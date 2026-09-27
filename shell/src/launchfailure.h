// SPDX-License-Identifier: MIT
// The Dock launch-failure notification (T-11.1b).
//
// T-10 rendered a transient "failed" badge on the Dock tile. T-11.1b replaces
// it with a real notification raised through the notification service, so the
// failure lands in the notification center and is visible even after the tile
// mark clears. This is a pure formatter over the `NotificationClient` seam so
// it is unit-testable with `MockNotificationClient`.
#pragma once

#include <QString>

class NotificationClient;

// Raise the launch-failure notification for `appName` (the display name of
// the app that failed; empty falls back to "app") with the human-readable
// `reason`. A no-op when `client` is null.
void raiseDockLaunchFailure(NotificationClient *client, const QString &appName,
                            const QString &reason);

// Raise a Dock operation notice (T-14.7f): a failed move to Trash or
// Downloads, or a partial multi-item failure, made user-visible through the
// same notification path as a launch failure. A no-op when `client` is null.
void raiseDockNotice(NotificationClient *client, const QString &summary,
                     const QString &body);