use anyhow::Result;
use chrono::Utc;
use native_db::watch::Event;
use std::sync::Arc;

use crate::db::Database;
use crate::{EventType, PackageVersion, TimelineEvent};
use crate::websocket::TimelineBroadcaster;

/// Spawns a background task that listens for PackageVersion inserts
/// and automatically creates timeline events for them.
pub fn spawn_package_version_listener(
    db: Arc<Database>,
    broadcaster: Arc<TimelineBroadcaster>,
) -> Result<()> {
    // Watch for all PackageVersion inserts
    let (recv, _watch_id) = db.db.watch().scan().primary().all::<PackageVersion>()?;

    tracing::info!("Started database listener for PackageVersion events");

    // Spawn a background task to process events
    tokio::spawn(async move {
        loop {
            match recv.recv() {
                Ok(event) => {
                    if let Err(e) =
                        handle_package_version_event(event, db.clone(), broadcaster.clone()).await
                    {
                        tracing::error!("Error handling package version event: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Error receiving watch event: {}", e);
                    // If the channel is disconnected, break the loop
                    break;
                }
            }
        }
        tracing::warn!("Database listener for PackageVersion events stopped");
    });

    Ok(())
}

async fn handle_package_version_event(
    event: Event,
    db: Arc<Database>,
    broadcaster: Arc<TimelineBroadcaster>,
) -> Result<()> {
    // Only handle Insert events (new versions)
    let version: PackageVersion = match event {
        Event::Insert(insert_event) => insert_event.inner()?,
        Event::Update(_) | Event::Delete(_) => {
            // We don't create timeline events for updates or deletes
            return Ok(());
        }
    };

    tracing::debug!(
        "Detected new PackageVersion insert: package_id={}, version={}",
        version.package_id,
        version.version
    );

    // Get the package to retrieve its name
    let package = match db.get_package(version.package_id)? {
        Some(pkg) => pkg,
        None => {
            tracing::warn!(
                "Package {} not found for version {}",
                version.package_id,
                version.version
            );
            return Ok(());
        }
    };

    let now = Utc::now();

    // Create timeline events for subscribed users
    match db.get_users_subscribed_to(&package.name) {
        Ok(subscribed_users) => {
            for user_id in subscribed_users {
                let event = TimelineEvent {
                    id: 0,
                    package_id: package.id,
                    user_id: Some(user_id),
                    event_type: EventType::NewRelease,
                    package_name: package.name.clone(),
                    version: Some(version.version.clone()),
                    message: format!("New version {} released", version.version),
                    metadata: None,
                    created_at: now,
                    notified_at: None,
                };

                match db.insert_timeline_event(event) {
                    Ok(saved_event) => {
                        // Broadcast the event to connected WebSocket clients
                        broadcaster.broadcast(saved_event);
                        tracing::debug!(
                            "Created timeline event for user {} for {} {}",
                            user_id,
                            package.name,
                            version.version
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to create timeline event for user {}: {}",
                            user_id,
                            e
                        );
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get subscribed users for {}: {}", package.name, e);
        }
    }

    // Broadcast a global event to WebSocket clients (not stored in database)
    let global_event = TimelineEvent {
        id: 0,
        package_id: package.id,
        user_id: None,
        event_type: EventType::NewRelease,
        package_name: package.name.clone(),
        version: Some(version.version.clone()),
        message: format!("New version {} released", version.version),
        metadata: None,
        created_at: now,
        notified_at: None,
    };

    // Broadcast the global event to connected WebSocket clients
    broadcaster.broadcast(global_event);
    tracing::info!(
        "Broadcast global timeline event for {} {}",
        package.name,
        version.version
    );

    Ok(())
}
