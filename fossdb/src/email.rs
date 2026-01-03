use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use once_cell::sync::Lazy;
use tera::{Context, Tera};

use crate::config::Config;

static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    let mut tera = Tera::default();

    tera.add_raw_template(
        "new_release.html",
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; line-height: 1.6; color: #333; }
        .container { max-width: 600px; margin: 0 auto; padding: 20px; }
        .header { background: #0066cc; color: white; padding: 20px; text-align: center; }
        .content { background: #f4f4f4; padding: 20px; margin-top: 20px; }
        .version { font-size: 24px; font-weight: bold; color: #0066cc; }
        .footer { margin-top: 20px; font-size: 12px; color: #666; }
        a { color: #0066cc; text-decoration: none; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>New Release Available</h1>
        </div>
        <div class="content">
            <p>Hello!</p>
            <p>A new version of <strong>{{ package_name }}</strong> has been released:</p>
            <p class="version">{{ version }}</p>
            <p>Released on: {{ release_date }}</p>
            {% if description %}
            <p>{{ description }}</p>
            {% endif %}
            <p><a href="{{ package_url }}">View package details</a></p>
        </div>
        <div class="footer">
            <p>You're receiving this because you're subscribed to {{ package_name }}.</p>
            <p><a href="{{ settings_url }}">Manage notification settings</a></p>
        </div>
    </div>
</body>
</html>
"#,
    )
    .unwrap();

    tera.add_raw_template(
        "new_release.txt",
        r#"
New Release: {{ package_name }} {{ version }}

A new version of {{ package_name }} has been released:

Version: {{ version }}
Released: {{ release_date }}

{% if description %}
{{ description }}
{% endif %}

View package details: {{ package_url }}

---
You're receiving this because you're subscribed to {{ package_name }}.
Manage settings: {{ settings_url }}
"#,
    )
    .unwrap();

    tera
});

pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    config: Config,
}

impl EmailService {
    pub fn new(config: Config) -> Result<Self> {
        let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)?
            .credentials(creds)
            .port(config.smtp_port)
            .build();

        let from = format!("{} <{}>", config.smtp_from_name, config.smtp_from_address).parse()?;

        Ok(Self {
            mailer,
            from,
            config,
        })
    }

    pub async fn send_new_release_notification(
        &self,
        to_email: &str,
        package_name: &str,
        version: &str,
        release_date: &str,
        description: Option<&str>,
    ) -> Result<()> {
        if !self.config.email_enabled {
            tracing::info!("Email disabled, skipping notification to {}", to_email);
            return Ok(());
        }

        let mut context = Context::new();
        context.insert("package_name", package_name);
        context.insert("version", version);
        context.insert("release_date", release_date);
        context.insert("description", &description.unwrap_or(""));
        context.insert(
            "package_url",
            &format!("https://fossdb.org/packages/{}", package_name),
        );
        context.insert("settings_url", "https://fossdb.org/settings");

        let html_body = TEMPLATES.render("new_release.html", &context)?;
        let text_body = TEMPLATES.render("new_release.txt", &context)?;

        let email = Message::builder()
            .from(self.from.clone())
            .to(to_email.parse()?)
            .subject(format!("New release: {} {}", package_name, version))
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text_body),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body),
                    ),
            )?;

        self.mailer.send(email).await?;

        tracing::info!(
            "Sent notification to {} for {} {}",
            to_email,
            package_name,
            version
        );
        Ok(())
    }
}
