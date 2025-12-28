use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;

use crate::{
    db::Database,
    email::EmailService,
};

pub struct NotificationProcessor {
    db: Arc<Database>,
    email: Arc<EmailService>,
}

impl NotificationProcessor {
    pub fn new(db: Arc<Database>, email: Arc<EmailService>) -> Self {
        Self { db, email }
    }

    pub async fn process_new_releases(&self) -> Result<()> {
        tracing::info!("Processing new release notifications...");

        // Get all pending notifications
        let pending_events = self.db.get_pending_notifications()?;

        if pending_events.is_empty() {
            tracing::debug!("No pending notifications to process");
            return Ok(());
        }

        tracing::info!("Found {} pending notification(s)", pending_events.len());

        let mut notifications_sent = 0;
        let mut notifications_skipped = 0;

        for mut event in pending_events {
            // Get the user for this event
            let user_id = match event.user_id {
                Some(id) => id,
                None => {
                    tracing::warn!("Event {} has no user_id, skipping", event.id);
                    continue;
                }
            };

            let user = match self.db.get_user(user_id) {
                Ok(Some(u)) => u,
                Ok(None) => {
                    tracing::warn!("User {} not found for event {}, skipping", user_id, event.id);
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to get user {} for event {}: {}", user_id, event.id, e);
                    continue;
                }
            };

            // Check if user has notifications enabled
            if !user.notifications_enabled {
                tracing::debug!("User {} has notifications disabled, skipping", user.id);
                notifications_skipped += 1;
                continue;
            }

            // Get package details
            let package = match self.db.get_package(event.package_id) {
                Ok(Some(p)) => p,
                Ok(None) => {
                    tracing::warn!("Package {} not found for event {}, skipping", event.package_id, event.id);
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to get package {} for event {}: {}", event.package_id, event.id, e);
                    continue;
                }
            };

            let version_string = "unknown".to_string();
            let version = event.version.as_ref().unwrap_or(&version_string);
            let release_date = event.created_at.format("%Y-%m-%d %H:%M UTC").to_string();

            // Send email
            match self.email.send_new_release_notification(
                &user.email,
                &event.package_name,
                version,
                &release_date,
                package.description.as_deref(),
            ).await {
                Ok(()) => {
                    // Mark notification as sent
                    event.notified_at = Some(Utc::now());

                    if let Err(e) = self.db.update_timeline_event(event.clone()) {
                        tracing::error!("Failed to update timeline event {}: {}", event.id, e);
                    } else {
                        notifications_sent += 1;
                        tracing::info!(
                            "Sent notification to {} for {} {}",
                            user.email,
                            event.package_name,
                            version
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to send email to {} for {} {}: {}",
                        user.email,
                        event.package_name,
                        version,
                        e
                    );
                    // Don't update notified_at - will retry next run
                }
            }

            // Rate limiting: small delay between emails to avoid overwhelming SMTP
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        tracing::info!(
            "Notification processing complete: {} sent, {} skipped",
            notifications_sent,
            notifications_skipped
        );

        Ok(())
    }
}
