import type { DocChapter } from "../docTypes";

const WEB_UI_PORT = 8025;

/** Mailpit usage documentation. `port` is the SMTP port. */
export function buildMailpitDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet Mailpit",
      navHint: "Email sandbox · why you need it",
      title: "What is Mailpit",
      intro:
        "Mailpit is a local email testing tool. It pretends to be an SMTP server and accepts every message your application sends, but never actually delivers any of them — they stay local for you to inspect.",
      blocks: [
        {
          kind: "text",
          value:
            "When debugging signup verification, password recovery, or order-notification emails during development, wiring up a real email service creates a chain of headaches: you need an account and an app password, sending has rate limits, messages easily get flagged as spam, and worse, test emails can be sent to real users by mistake. Mailpit solves all of this at once — just point your app's SMTP address at it, no code changes required.",
        },
        {
          kind: "text",
          value: "What it can do:",
        },
        {
          kind: "list",
          items: [
            "Accept mail from any sender to any recipient, without delivering it.",
            "Fully render HTML emails, and also show the plain-text version and the raw message.",
            "View attachments, CC/BCC, and all kinds of headers.",
            "Expose an HTTP API so automated tests can assert \"that verification email really was sent, and here's the code\".",
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Development and testing only",
          value:
            "Mailpit does not deliver messages to real recipients, so it must never be configured in production. Use a real email provider in production and put SMTP settings in environment variables so the two setups don't get mixed up.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Ports · verifying delivery",
      title: "Two ports, each with its own job",
      intro:
        "Mailpit opens two ports at the same time: one for receiving mail, one for viewing it. Make sure you don't mix them up when configuring your app.",
      blocks: [
        {
          kind: "table",
          head: ["Purpose", "Address", "Notes"],
          rows: [
            [
              "SMTP inbound",
              `127.0.0.1:${port}`,
              "Point your app's mail config here; this is where messages are received",
            ],
            [
              "Web UI / API",
              `http://127.0.0.1:${WEB_UI_PORT}`,
              "Open in a browser to view the inbox; also the base URL for the HTTP API",
            ],
            ["Username / password", "(empty)", "The local instance requires no authentication by default"],
            ["Encryption", "No TLS", "Plaintext local connection is fine; remember to turn off STARTTLS"],
          ],
        },
        {
          kind: "list",
          items: [
            "In the Overview tab, confirm the status is Running.",
            "Change your app's SMTP config to the address above and restart the app.",
            "Trigger a send, then go back to the Inbox tab — the message will show up immediately.",
          ],
        },
        {
          kind: "text",
          value:
            "If you don't want to change code yet, send a test email from the command line first to confirm the pipe is working:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Verify from the command line",
          code: `# Option 1: use curl to talk SMTP directly
printf 'From: dev@demo.local\\r\\n'\\
'To: user@demo.local\\r\\n'\\
'Subject: Mailpit test\\r\\n'\\
'\\r\\n'\\
'This is a test message\\r\\n' > /tmp/mail.txt

curl --url "smtp://127.0.0.1:${port}" \\
     --mail-from "dev@demo.local" \\
     --mail-rcpt "user@demo.local" \\
     --upload-file /tmp/mail.txt

# Option 2: just check the port is reachable
nc -zv 127.0.0.1 ${port}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Any recipient address works",
          value:
            "Mailpit doesn't validate that the domain exists and never tries to deliver, so a non-existent address like user@demo.local is fine. That's exactly the point — you can safely use any address for testing.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language integration",
      navHint: "Java · Go · TS · Python",
      title: "Point your app's mailer here",
      intro: `All of the following configurations point at the local SMTP server 127.0.0.1:${port}. There are only three things that matter: correct host and port, no TLS, no username or password.`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot: pom.xml",
              code: `<dependency>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter-mail</artifactId>
</dependency>`,
            },
            {
              label: "Java",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  mail:
    host: 127.0.0.1
    port: ${port}
    # Local Mailpit does not require a username or password; leave them empty
    username:
    password:
    properties:
      mail:
        smtp:
          auth: false
          starttls:
            enable: false`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Send an HTML email",
              code: `@Service
public class MailService {

    private final JavaMailSender sender;

    public MailService(JavaMailSender sender) {
        this.sender = sender;
    }

    public void sendVerifyCode(String to, String code) throws MessagingException {
        MimeMessage message = sender.createMimeMessage();
        MimeMessageHelper helper = new MimeMessageHelper(message, true, "UTF-8");

        helper.setFrom("noreply@demo.local");
        helper.setTo(to);
        helper.setSubject("Your verification code");
        // The second argument true means the content is HTML
        helper.setText("<h1>Code: " + code + "</h1><p>Valid for 5 minutes</p>", true);

        sender.send(message);
    }
}`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Standard library net/smtp",
              code: `package mailer

import (
    "fmt"
    "net/smtp"
)

const smtpAddr = "127.0.0.1:${port}"

func SendVerifyCode(to, code string) error {
    from := "noreply@demo.local"

    msg := fmt.Sprintf(
        "From: %s\\r\\n"+
            "To: %s\\r\\n"+
            "Subject: Your verification code\\r\\n"+
            "MIME-Version: 1.0\\r\\n"+
            "Content-Type: text/html; charset=UTF-8\\r\\n"+
            "\\r\\n"+
            "<h1>Code: %s</h1><p>Valid for 5 minutes</p>\\r\\n",
        from, to, code)

    // The second argument is auth; local Mailpit needs no auth, so pass nil
    return smtp.SendMail(smtpAddr, nil, from, []string{to}, []byte(msg))
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "Install nodemailer",
              code: `npm install nodemailer`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Send mail with nodemailer",
              code: `import nodemailer from "nodemailer";

// Reuse a single module-level instance
export const transporter = nodemailer.createTransport({
  host: "127.0.0.1",
  port: ${port},
  secure: false,      // Do not use SSL
  ignoreTLS: true,    // Local Mailpit doesn't support STARTTLS, skip it
  // Do not set the auth field
});

export async function sendVerifyCode(to: string, code: string) {
  await transporter.sendMail({
    from: '"Demo" <noreply@demo.local>',
    to,
    subject: "Your verification code",
    text: "Code: " + code + ", valid for 5 minutes",
    html: "<h1>Code: " + code + "</h1><p>Valid for 5 minutes</p>",
  });
}`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Standard library smtplib",
              code: `import smtplib
from email.message import EmailMessage

SMTP_HOST = "127.0.0.1"
SMTP_PORT = ${port}


def send_verify_code(to: str, code: str) -> None:
    msg = EmailMessage()
    msg["From"] = "noreply@demo.local"
    msg["To"] = to
    msg["Subject"] = "Your verification code"

    msg.set_content(f"Code: {code}, valid for 5 minutes")
    msg.add_alternative(
        f"<h1>Code: {code}</h1><p>Valid for 5 minutes</p>",
        subtype="html",
    )

    # Local Mailpit needs neither starttls nor login
    with smtplib.SMTP(SMTP_HOST, SMTP_PORT, timeout=5) as smtp:
        smtp.send_message(msg)`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Getting \"STARTTLS failed\" or \"auth failed\"?",
          value:
            "These errors almost always mean production mail settings were copied over with just the port changed. Mailpit neither supports TLS nor requires authentication — confirm STARTTLS is off, set auth to false, and clear the username and password.",
        },
      ],
    },

    {
      id: "testing",
      navLabel: "Automated testing",
      navHint: "HTTP API",
      title: "Assert on email content in tests",
      intro:
        "Mailpit's web UI also exposes an HTTP API, which makes end-to-end tests of the form \"send signup request → assert a verification email arrived → extract the code and continue the flow\" straightforward to write.",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "Common API calls",
          code: `BASE=http://127.0.0.1:${WEB_UI_PORT}

# List messages (newest first), returns JSON
curl -s "$BASE/api/v1/messages?limit=10"

# Search: filter by recipient, subject, and more
curl -s "$BASE/api/v1/search?query=to:user@demo.local"

# Fetch a single message in full; ID comes from the list above
curl -s "$BASE/api/v1/message/{ID}"

# Clear the inbox to isolate test cases
curl -s -X DELETE "$BASE/api/v1/messages"`,
        },
        {
          kind: "code",
          lang: "typescript",
          caption: "Extract a verification code in tests",
          code: `const BASE = "http://127.0.0.1:${WEB_UI_PORT}";

// Clear before each test case to avoid interference
export async function clearMailbox() {
  await fetch(BASE + "/api/v1/messages", { method: "DELETE" });
}

export async function latestCodeFor(email: string): Promise<string> {
  const res = await fetch(
    BASE + "/api/v1/search?query=to:" + encodeURIComponent(email),
  );
  const data = await res.json();
  if (!data.messages?.length) throw new Error("No email received");

  const detail = await fetch(BASE + "/api/v1/message/" + data.messages[0].ID);
  const body = await detail.json();

  const matched = String(body.Text ?? body.HTML).match(/\\d{4,6}/);
  if (!matched) throw new Error("No verification code found in the email");
  return matched[0];
}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Mind the async timing",
          value:
            "There is a very short delay between the app sending and Mailpit receiving, so an assertion made immediately afterward can flake. Prefer a polling helper with retries (e.g. every 100ms for up to 3 seconds) over a fixed sleep — it is both faster and more reliable.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Common issues",
      navHint: "Troubleshooting checklist",
      title: "When no email arrives, check these in order",
      intro: "Almost every \"I sent it but the inbox is empty\" case has its cause in the table below.",
      blocks: [
        {
          kind: "table",
          head: ["Symptom", "Likely cause", "What to do"],
          rows: [
            [
              "Connection refused",
              "The service isn't running, or the port was set to the Web UI port",
              `Confirm Running in the Overview tab; SMTP is ${port}, not ${WEB_UI_PORT}`,
            ],
            [
              "STARTTLS-related error",
              "The client is forcing encryption",
              "Turn off starttls / secure; a plain local connection is enough",
            ],
            [
              "Authentication failed",
              "A username and password are configured",
              "Clear username and password, and set auth to false",
            ],
            [
              "Send succeeds but inbox is empty",
              "The app is connecting to a different SMTP, or the message is stuck in an async queue",
              "Confirm the config is actually applied; check whether the mail is queued but not sent",
            ],
            [
              "Garbled non-ASCII subject",
              "No charset specified",
              "Explicitly declare UTF-8 in both headers and body",
            ],
            [
              "Mailbox keeps growing",
              "Mailpit retains history by default",
              "Clear from the UI periodically, or call the DELETE endpoint",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do this in Zhiyu",
          value:
            "The Inbox tab lets you browse messages, view the HTML rendering, and see the raw source; the Runtime Logs tab shows SMTP-level connection records, which is very useful for diagnosing connection failures; on port conflicts, adjust things in the Config File tab.",
        },
      ],
    },
  ];
}
