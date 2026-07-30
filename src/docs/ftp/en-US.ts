import type { DocChapter } from "../docTypes";

export function buildFtpDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "About FTP",
      navHint: "Local file transfer",
      title: "Zhiyu FTP Server",
      intro:
        "Zhiyu runs SFTPGo in portable FTP mode. It is a standalone binary with no system-wide dependencies and listens on localhost only.",
      blocks: [
        {
          kind: "table",
          head: ["Item", "Default", "Notes"],
          rows: [
            ["Endpoint", `127.0.0.1:${port}`, "Local access only"],
            ["Username", "zhiyu", "Local development account"],
            ["Shared folder", "~/.devbox/instances/ftp/default/data", "Uploaded files are stored here"],
            ["Passive ports", "50000–50009", "Directory listings and transfers"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Unencrypted protocol",
          value:
            "Plain FTP does not encrypt credentials or file contents. Keep the binding on 127.0.0.1 and never expose this development service to the public internet.",
        },
      ],
    },
    {
      id: "use",
      navLabel: "Connect & transfer",
      navHint: "curl · clients",
      title: "Upload and download files",
      intro: "Connect with FileZilla, Cyberduck, curl, or a language standard library.",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "Upload and download with curl",
          code: `curl --ftp-pasv -T ./demo.txt \\
  "ftp://zhiyu:zhiyu-local-ftp-2026@127.0.0.1:${port}/"

curl --ftp-pasv \\
  "ftp://zhiyu:zhiyu-local-ftp-2026@127.0.0.1:${port}/demo.txt" \\
  -o demo.txt`,
        },
        {
          kind: "list",
          items: [
            "Copy the endpoint and credentials from the Connection tab.",
            "Use passive mode (PASV) in desktop FTP clients.",
            "Files remain in the shared folder after the service stops.",
          ],
        },
      ],
    },
    {
      id: "troubleshooting",
      navLabel: "Troubleshooting",
      navHint: "Ports · logs",
      title: "When a connection fails",
      intro: "FTP uses a control port plus data ports. Passive-port conflicts commonly break directory listings.",
      blocks: [
        {
          kind: "list",
          items: [
            "Use Port Inspector to check whether port 2121 is occupied.",
            "If login works but listing fails, check ports 50000–50009.",
            "Inspect stderr.log in the Logs tab for startup failures.",
            "Restart the service after changing configuration.",
          ],
        },
      ],
    },
  ];
}
