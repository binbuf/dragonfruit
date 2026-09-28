#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Generate per-image reference notes with the vision model from the Symphony config.

The art-direction captures under ``docs/reference/macos/`` are local-only and
git-ignored, but the panes, labels, controls, and layout we clone from them
should not be eyeballed. This script runs every image in a folder through the
same vision model the Symphony harness uses (the ``vision`` stanza in
``.symphony/symphony.config.json``) and writes a sibling ``<name>.md`` next to
each capture, exactly like the existing ``Dock.md``.

It reads model, provider, API-key env var, timeout, image-size cap, and prompt
straight from the config so the notes stay reproducible as the config changes.
A different prompt (for example, a strict label/component inventory) can be
supplied with ``--prompt`` / ``--prompt-file`` without editing the config.

Usage::

    # Everything in the reference folder except Dock.png.
    python3 scripts/vision-reference-notes.py

    # One image, overwriting any existing note.
    python3 scripts/vision-reference-notes.py docs/reference/macos/SystemSettings_Bluetooth.png --force

    # Dry run: show what would be processed.
    python3 scripts/vision-reference-notes.py --list

Only Pillow and the standard library are required (the sibling
``measure-settings-reference.py`` already depends on Pillow).
"""

from __future__ import annotations

import argparse
import base64
import io
import json
import os
import random
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone

try:
    from PIL import Image
except ImportError:  # pragma: no cover - Pillow is an assumed dev dependency
    Image = None

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_CONFIG = os.path.join(REPO_ROOT, ".symphony", "symphony.config.json")
DEFAULT_DIR = os.path.join(REPO_ROOT, "docs", "reference", "macos")
DEFAULT_BASE_URL = "https://openrouter.ai/api/v1/chat/completions"
IMAGE_EXTENSIONS = {
    ".png",
    ".jpg",
    ".jpeg",
    ".webp",
    ".gif",
    ".bmp",
    ".tif",
    ".tiff",
}


def log(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def load_vision_config(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as handle:
            config = json.load(handle)
    except FileNotFoundError:
        raise SystemExit(f"config not found: {path}")
    except json.JSONDecodeError as exc:
        raise SystemExit(f"config is not valid JSON: {path}: {exc}")

    vision = config.get("vision") or {}
    if not vision:
        raise SystemExit(f"no \"vision\" stanza in {path}")
    if not vision.get("enabled", True):
        log(f"warning: vision.enabled is false in {path}; running anyway")
    return vision


def is_image(path: str) -> bool:
    extension = os.path.splitext(path)[1].lower()
    if extension in IMAGE_EXTENSIONS:
        return True
    # Some captures were saved without an extension (e.g.
    # "SystemSettings_Accessibilitypng"). Fall back to sniffing the magic bytes.
    if Image is None:
        return False
    try:
        with Image.open(path) as image:
            image.verify()
        return True
    except Exception:
        return False


def collect_targets(args, vision: dict) -> list[str]:
    excludes = {os.path.basename(name) for name in args.exclude}
    targets: list[str] = []
    for raw in args.targets:
        for path in sorted(_expand(raw)):
            name = os.path.basename(path)
            if name in excludes:
                continue
            if name.startswith("."):
                continue
            if os.path.splitext(name)[1].lower() == ".md":
                continue
            if not is_image(path):
                continue
            note = note_path(path)
            if os.path.exists(note) and not args.force:
                log(f"skip (note exists): {os.path.relpath(path, REPO_ROOT)}")
                continue
            targets.append(path)
    return targets


def _expand(raw: str) -> list[str]:
    from glob import glob

    if os.path.isdir(raw):
        return [
            os.path.join(raw, name)
            for name in os.listdir(raw)
            if os.path.isfile(os.path.join(raw, name))
        ]
    matches = sorted(glob(raw))
    if matches:
        return matches
    if os.path.exists(raw):
        return [raw]
    raise SystemExit(f"target matched nothing: {raw}")


def note_path(image_path: str) -> str:
    directory, name = os.path.split(image_path)
    # Strip leading/trailing whitespace from one capture named with a stray
    # leading space so the note is easy to reference; the image is untouched.
    stem = os.path.splitext(name.strip())[0]
    return os.path.join(directory, stem + ".md")


def prepare_image(path: str, max_bytes: int) -> tuple[str, str]:
    """Return (data-url, mime) for the image, downscaling if it is too large."""
    if Image is None:
        raise SystemExit("Pillow is required to read images")

    with Image.open(path) as image:
        mime = Image.MIME.get(image.format or "", "image/png")
        image.load()

        def encode(img) -> bytes:
            buffer = io.BytesIO()
            fmt = {"image/jpeg": "JPEG"}.get(mime, "PNG")
            if fmt == "JPEG" and img.mode not in ("RGB", "L"):
                img = img.convert("RGB")
            img.save(buffer, format=fmt)
            return buffer.getvalue()

        raw = encode(image)
        # base64 inflates by ~4/3; keep the *encoded* payload under the cap.
        if len(raw) * 4 // 3 > max_bytes:
            scale = 0.85
            working = image.copy()
            while scale > 0.15:
                resized = working.resize(
                    (max(1, int(working.width * scale)), max(1, int(working.height * scale))),
                    Image.LANCZOS,
                )
                raw = encode(resized)
                if len(raw) * 4 // 3 <= max_bytes:
                    log(
                        f"downscaled {os.path.basename(path)} "
                        f"{image.width}x{image.height} -> {resized.width}x{resized.height}"
                    )
                    break
                scale -= 0.1
            else:
                raise SystemExit(f"could not shrink {path} under {max_bytes} bytes")

        data_url = f"data:{mime};base64,{base64.b64encode(raw).decode('ascii')}"
        return data_url, mime


def call_vision(
    *,
    base_url: str,
    api_key: str,
    model: str,
    prompt: str,
    data_url: str,
    timeout: float,
    attempts: int,
    base_sec: float,
    factor: float,
    max_sec: float,
    jitter: float,
    max_backoff: float,
    provider: dict | None = None,
) -> str:
    payload = {
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": data_url}},
                ],
            }
        ],
        "temperature": 0.2,
        "max_tokens": 8000,
    }
    if provider:
        payload["provider"] = provider
    body = json.dumps(payload).encode("utf-8")
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "HTTP-Referer": "https://github.com/dragonfruit",
        "X-Title": "dragonfruit reference notes",
    }

    delay = base_sec
    last_error: str | None = None
    for attempt in range(1, attempts + 1):
        request = urllib.request.Request(base_url, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                parsed = json.loads(response.read().decode("utf-8"))
            return parsed["choices"][0]["message"]["content"]
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace")[:400]
            last_error = f"HTTP {exc.code}: {detail}"
            retry_after = exc.headers.get("Retry-After")
            if retry_after and retry_after.isdigit():
                delay = float(retry_after)
            # 4xx other than rate limiting will not get better by retrying.
            if 400 <= exc.code < 500 and exc.code != 429:
                break
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, KeyError) as exc:
            last_error = f"{type(exc).__name__}: {exc}"

        if attempt < attempts:
            sleep_for = min(delay, max_backoff) + random.uniform(0, jitter * delay)
            log(f"  retry {attempt}/{attempts - 1} in {sleep_for:.0f}s ({last_error})")
            time.sleep(sleep_for)
            delay = min(delay * factor, max_sec)

    raise RuntimeError(last_error or "vision request failed")


def process_one(path: str, args, vision: dict, retry: dict) -> tuple[str, str | None]:
    name = os.path.basename(path)
    started = time.time()
    try:
        data_url, _ = prepare_image(path, args.max_image_bytes)
        provider = None
        if args.provider_order or args.provider_sort or not args.no_fallbacks:
            provider = {"allow_fallbacks": not args.no_fallbacks}
            if args.provider_order:
                provider["order"] = [p.strip() for p in args.provider_order.split(",") if p.strip()]
            if args.provider_sort:
                provider["sort"] = args.provider_sort
        text = call_vision(
            base_url=args.base_url,
            api_key=args.api_key,
            model=args.model,
            prompt=args.prompt,
            data_url=data_url,
            timeout=args.timeout,
            attempts=args.attempts,
            base_sec=retry.get("baseSec", 30),
            factor=retry.get("factor", 2),
            max_sec=retry.get("maxSec", 900),
            jitter=retry.get("jitter", 0.2),
            max_backoff=args.max_backoff,
            provider=provider,
        )
        out = note_path(path)
        if not args.dry_run:
            temp = out + ".tmp"
            with open(temp, "w", encoding="utf-8") as handle:
                handle.write(text.strip() + "\n")
            os.replace(temp, out)
        elapsed = time.time() - started
        return name, f"{elapsed:.0f}s -> {os.path.relpath(out, REPO_ROOT)} ({len(text)} chars)"
    except Exception as exc:  # noqa: BLE001 - report and continue the batch
        return name, f"FAILED: {exc}"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "targets",
        nargs="*",
        default=[DEFAULT_DIR],
        help=f"files, globs, or directories (default: {os.path.relpath(DEFAULT_DIR, REPO_ROOT)})",
    )
    parser.add_argument("--config", default=DEFAULT_CONFIG, help="Symphony config with the vision stanza")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL, help="chat-completions endpoint")
    parser.add_argument("--api-key", help="API key (default: config vision.apiKeyEnv)")
    parser.add_argument("--model", help="model id (default: config vision.model)")
    parser.add_argument("--prompt", help="prompt (default: config vision.prompt)")
    parser.add_argument("--prompt-file", help="read the prompt from a file (overrides --prompt)")
    parser.add_argument(
        "--provider-order",
        help="comma-separated upstream provider order (default: config vision.providerOrder)",
    )
    parser.add_argument(
        "--provider-sort",
        help="OpenRouter provider sort, e.g. throughput or latency (default: config vision.providerSort)",
    )
    parser.add_argument("--no-fallbacks", action="store_true", help="do not allow provider fallbacks")
    parser.add_argument("--timeout", type=float, help="per-request timeout seconds (default: config vision.timeoutMs)")
    parser.add_argument(
        "--max-image-bytes",
        type=int,
        help="encoded-payload cap; larger images are downscaled (default: config vision.maxImageBytes)",
    )
    parser.add_argument("--exclude", action="append", default=[], help="basename to skip (repeatable; Dock.png is always skipped)")
    parser.add_argument("-j", "--concurrency", type=int, default=4, help="parallel requests (default: 4)")
    parser.add_argument("-f", "--force", action="store_true", help="overwrite existing notes")
    parser.add_argument("-n", "--dry-run", action="store_true", help="do not write notes")
    parser.add_argument("-l", "--list", action="store_true", help="list targets and exit")
    parser.add_argument("--limit", type=int, help="process at most N images")
    parser.add_argument("--attempts", type=int, help="max attempts per image (default: config retry.maxAttempts)")
    parser.add_argument("--max-backoff", type=float, default=60.0, help="cap on retry sleep seconds (default: 60)")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    vision = load_vision_config(args.config)

    try:
        with open(args.config, "r", encoding="utf-8") as handle:
            retry = (json.load(handle).get("retry") or {})
    except Exception:
        retry = {}

    if not args.prompt_file and not args.prompt:
        args.prompt = vision.get("prompt")
    if args.prompt_file:
        with open(args.prompt_file, "r", encoding="utf-8") as handle:
            args.prompt = handle.read().strip()
    if not args.prompt:
        raise SystemExit("no prompt: set vision.prompt in the config or pass --prompt/--prompt-file")

    if not args.model:
        args.model = vision.get("model")
    if not args.model:
        raise SystemExit("no model: set vision.model in the config or pass --model")

    if not args.provider_order and vision.get("providerOrder"):
        args.provider_order = ",".join(vision["providerOrder"])
    if not args.provider_sort and vision.get("providerSort"):
        args.provider_sort = vision["providerSort"]

    if not args.api_key:
        key_env = vision.get("apiKeyEnv", "OPENROUTER_API_KEY")
        args.api_key = os.environ.get(key_env, "")
        if not args.api_key:
            raise SystemExit(f"no API key: set ${key_env} or pass --api-key")

    if args.timeout is None:
        args.timeout = vision.get("timeoutMs", 300000) / 1000.0
    if args.max_image_bytes is None:
        args.max_image_bytes = vision.get("maxImageBytes", 20 * 1024 * 1024)
    if args.attempts is None:
        args.attempts = max(1, int(retry.get("maxAttempts", 4)))

    args.exclude.append("Dock.png")
    targets = collect_targets(args, vision)
    if args.limit:
        targets = targets[: args.limit]

    if args.list or not targets:
        for path in targets:
            print(os.path.relpath(path, REPO_ROOT))
        if not targets:
            log("nothing to do")
        return 0

    log(
        f"vision: {args.model} via {args.base_url} | {len(targets)} image(s), "
        f"concurrency {args.concurrency}, timeout {args.timeout:.0f}s"
    )

    failures = 0
    with ThreadPoolExecutor(max_workers=max(1, args.concurrency)) as pool:
        futures = {pool.submit(process_one, path, args, vision, retry): path for path in targets}
        for future in as_completed(futures):
            name, status = future.result()
            if status and status.startswith("FAILED"):
                failures += 1
            log(f"[{name}] {status}")

    log(f"done: {len(targets) - failures}/{len(targets)} written, {failures} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))