/* SPDX-License-Identifier: MIT
 *
 * A minimal SDL2 client for the T-14.6a strange-app zoo. Real SDL2 game
 * windows are a common Linux identity case (Wayland `app_id` from
 * `SDL_APP_ID`, or an X11 `WM_CLASS`), so the zoo runs this instead of a
 * specific commercial game. Build:
 *
 *   gcc -o sdl_zoo sdl_zoo.c $(pkg-config --cflags --libs sdl2)
 *
 * Run on Wayland against the Dragonfruit socket:
 *
 *   WAYLAND_DISPLAY=<socket> SDL_VIDEODRIVER=wayland \
 *     SDL_APP_ID=game.zoo.sdl ./sdl_zoo
 *
 * or on Xwayland with the SDL x11 driver (WM_CLASS from SDL_VIDEO_X11_WMCLASS).
 *
 * The window is drawn every iteration. SDL's Wayland backend does not attach
 * a buffer until the app presents a frame, and the compositor only maps a
 * toplevel once it has a buffer; a sample that never draws would therefore
 * never map (the T-14.6a gap). Painting is what makes the Wayland path real.
 */
#include <SDL2/SDL.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (SDL_Init(SDL_INIT_VIDEO) != 0) {
        fprintf(stderr, "sdl_zoo: SDL_Init failed: %s\n", SDL_GetError());
        return 1;
    }
    SDL_Window *win = SDL_CreateWindow("SDL Zoo Game",
        SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED, 400, 260, SDL_WINDOW_SHOWN);
    if (!win) {
        fprintf(stderr, "sdl_zoo: SDL_CreateWindow failed: %s\n", SDL_GetError());
        SDL_Quit();
        return 1;
    }
    Uint32 start = SDL_GetTicks();
    int running = 1;
    int lifetime_ms = 120000;
    for (int i = 1; i < argc - 1; i++) {
        if (SDL_strcmp(argv[i], "--lifetime-ms") == 0) {
            lifetime_ms = SDL_atoi(argv[i + 1]);
        }
    }
    while (running && (Sint32)(SDL_GetTicks() - start) < lifetime_ms) {
        SDL_Event e;
        while (SDL_PollEvent(&e)) {
            if (e.type == SDL_QUIT) {
                running = 0;
            }
        }
        /* Present a frame: SDL attaches its first wl_buffer here, which is
         * what lets the compositor map the toplevel. */
        SDL_Surface *surface = SDL_GetWindowSurface(win);
        if (surface) {
            Uint8 phase = (Uint8)((SDL_GetTicks() - start) / 16);
            SDL_FillRect(surface, NULL,
                SDL_MapRGB(surface->format, 40, (Uint8)(80 + phase), 200));
            SDL_UpdateWindowSurface(win);
        }
        SDL_Delay(50);
    }
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}