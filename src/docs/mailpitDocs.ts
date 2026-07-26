import type { DocChapter } from "./docTypes";

const WEB_UI_PORT = 8025;

/** Mailpit 使用文档。port 为 SMTP 端口。 */
export function buildMailpitDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Mailpit",
      navHint: "邮件沙箱 · 为什么需要",
      title: "Mailpit 是什么",
      intro:
        "Mailpit 是一个本地邮件测试工具。它伪装成一台 SMTP 服务器，接收你程序发出的所有邮件，但一封都不会真正投递出去，而是留在本地供你查看。",
      blocks: [
        {
          kind: "text",
          value:
            "开发阶段调试注册验证、找回密码、订单通知这类邮件时，如果直接接真实邮件服务，会遇到一连串麻烦：要申请账号和授权码、发信有频率限制、容易被判定成垃圾邮件、更糟的是可能把测试邮件误发给真实用户。Mailpit 把这些问题一次性解决——把应用的 SMTP 地址指向它就行，其他代码一行都不用改。",
        },
        {
          kind: "text",
          value: "它能做的事：",
        },
        {
          kind: "list",
          items: [
            "接收任意发件人、任意收件人的邮件，不做投递。",
            "完整还原 HTML 邮件的渲染效果，同时也能看纯文本版本和原始报文。",
            "查看附件、抄送密送、各类邮件头。",
            "提供 HTTP API，可以在自动化测试里断言「刚才那封验证码邮件确实发出去了，验证码是多少」。",
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "只用于开发和测试环境",
          value:
            "Mailpit 不会把邮件投递给真实收件人，所以绝对不能配置在生产环境。生产环境请使用真实的邮件服务商，并把 SMTP 配置做成环境变量，避免两套配置混淆。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "端口 · 收信验证",
      title: "两个端口，分别做什么",
      intro:
        "Mailpit 同时开着两个端口：一个收邮件，一个给你看邮件。配置应用时别填错。",
      blocks: [
        {
          kind: "table",
          head: ["用途", "地址", "说明"],
          rows: [
            [
              "SMTP 收信",
              `127.0.0.1:${port}`,
              "应用的邮件配置填这个，负责接收程序发出的邮件",
            ],
            [
              "Web 界面 / API",
              `http://127.0.0.1:${WEB_UI_PORT}`,
              "浏览器打开可以看收件箱，也是 HTTP API 的地址",
            ],
            ["用户名 / 密码", "（空）", "本地实例默认不要求认证"],
            ["加密", "无需 TLS", "本机明文连接即可，记得关掉 STARTTLS"],
          ],
        },
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "把应用的 SMTP 配置改成上面的地址，重启应用。",
            "触发一次发信，然后回到「邮件收件箱」标签页，邮件会立即出现。",
          ],
        },
        {
          kind: "text",
          value:
            "不想改代码也可以先用命令行发一封测试邮件，确认链路是通的：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "命令行验证",
          code: `# 方式一：用 curl 直接走 SMTP
printf 'From: dev@demo.local\\r\\n'\\
'To: user@demo.local\\r\\n'\\
'Subject: Mailpit 测试\\r\\n'\\
'\\r\\n'\\
'这是一封测试邮件\\r\\n' > /tmp/mail.txt

curl --url "smtp://127.0.0.1:${port}" \\
     --mail-from "dev@demo.local" \\
     --mail-rcpt "user@demo.local" \\
     --upload-file /tmp/mail.txt

# 方式二：确认端口通不通
nc -zv 127.0.0.1 ${port}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "收件人地址可以随便写",
          value:
            "Mailpit 不校验域名是否存在，也不会尝试投递，所以 user@demo.local 这种不存在的地址完全没问题。这正是它的价值——你可以放心用任意地址做测试。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "把应用的发信指过来",
      intro: `下面的配置都指向本机 SMTP 127.0.0.1:${port}。关键点只有三条：主机端口填对、不要开 TLS、不要配用户名密码。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot：pom.xml",
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
    # 本地 Mailpit 不需要用户名密码，留空即可
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
              caption: "发送 HTML 邮件",
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
        helper.setSubject("你的验证码");
        // 第二个参数 true 表示内容是 HTML
        helper.setText("<h1>验证码：" + code + "</h1><p>5 分钟内有效</p>", true);

        sender.send(message);
    }
}`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "标准库 net/smtp",
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
            "Subject: 你的验证码\\r\\n"+
            "MIME-Version: 1.0\\r\\n"+
            "Content-Type: text/html; charset=UTF-8\\r\\n"+
            "\\r\\n"+
            "<h1>验证码：%s</h1><p>5 分钟内有效</p>\\r\\n",
        from, to, code)

    // 第二个参数是 auth，本地 Mailpit 不需要认证，传 nil
    return smtp.SendMail(smtpAddr, nil, from, []string{to}, []byte(msg))
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 nodemailer",
              code: `npm install nodemailer`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "nodemailer 发信",
              code: `import nodemailer from "nodemailer";

// 模块级单例复用
export const transporter = nodemailer.createTransport({
  host: "127.0.0.1",
  port: ${port},
  secure: false,      // 不使用 SSL
  ignoreTLS: true,    // 本地 Mailpit 不支持 STARTTLS，直接跳过
  // 不要配 auth 字段
});

export async function sendVerifyCode(to: string, code: string) {
  await transporter.sendMail({
    from: '"Demo" <noreply@demo.local>',
    to,
    subject: "你的验证码",
    text: "验证码：" + code + "，5 分钟内有效",
    html: "<h1>验证码：" + code + "</h1><p>5 分钟内有效</p>",
  });
}`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "标准库 smtplib",
              code: `import smtplib
from email.message import EmailMessage

SMTP_HOST = "127.0.0.1"
SMTP_PORT = ${port}


def send_verify_code(to: str, code: str) -> None:
    msg = EmailMessage()
    msg["From"] = "noreply@demo.local"
    msg["To"] = to
    msg["Subject"] = "你的验证码"

    msg.set_content(f"验证码：{code}，5 分钟内有效")
    msg.add_alternative(
        f"<h1>验证码：{code}</h1><p>5 分钟内有效</p>",
        subtype="html",
    )

    # 本地 Mailpit 无需 starttls，也无需 login
    with smtplib.SMTP(SMTP_HOST, SMTP_PORT, timeout=5) as smtp:
        smtp.send_message(msg)`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "报「STARTTLS 失败」或「认证失败」怎么办",
          value:
            "这两类错误几乎都是因为把生产环境的邮件配置直接拿来改了个端口。Mailpit 既不支持 TLS 也不需要认证，请确认已经关掉 starttls、把 auth 设为 false，并清空用户名密码。",
        },
      ],
    },

    {
      id: "testing",
      navLabel: "自动化测试",
      navHint: "HTTP API",
      title: "在测试里断言邮件内容",
      intro:
        "Mailpit 的 Web 界面同时提供 HTTP API，这让「发注册请求 → 断言收到验证码邮件 → 取出验证码继续走流程」这种端到端测试变得很容易写。",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "常用 API",
          code: `BASE=http://127.0.0.1:${WEB_UI_PORT}

# 列出邮件（按时间倒序），返回 JSON
curl -s "$BASE/api/v1/messages?limit=10"

# 搜索：按收件人、标题等条件过滤
curl -s "$BASE/api/v1/search?query=to:user@demo.local"

# 取某封邮件的完整内容，ID 从上面的列表里拿
curl -s "$BASE/api/v1/message/{ID}"

# 清空收件箱，测试用例之间做隔离
curl -s -X DELETE "$BASE/api/v1/messages"`,
        },
        {
          kind: "code",
          lang: "typescript",
          caption: "在测试里取验证码",
          code: `const BASE = "http://127.0.0.1:${WEB_UI_PORT}";

// 每个用例开始前清空，避免相互干扰
export async function clearMailbox() {
  await fetch(BASE + "/api/v1/messages", { method: "DELETE" });
}

export async function latestCodeFor(email: string): Promise<string> {
  const res = await fetch(
    BASE + "/api/v1/search?query=to:" + encodeURIComponent(email),
  );
  const data = await res.json();
  if (!data.messages?.length) throw new Error("没有收到邮件");

  const detail = await fetch(BASE + "/api/v1/message/" + data.messages[0].ID);
  const body = await detail.json();

  const matched = String(body.Text ?? body.HTML).match(/\\d{4,6}/);
  if (!matched) throw new Error("邮件里没有找到验证码");
  return matched[0];
}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "注意异步时序",
          value:
            "应用发信和 Mailpit 收信之间有极短的延迟，测试里紧接着断言可能偶发失败。建议写一个带重试的轮询helper（比如每 100ms 查一次、最多等 3 秒），而不是固定 sleep，这样既稳定又快。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "常见问题",
      navHint: "排查清单",
      title: "收不到邮件时按这个顺序排查",
      intro: "绝大多数「发了但收件箱是空的」都能在下面这张表里找到原因。",
      blocks: [
        {
          kind: "table",
          head: ["现象", "可能原因", "怎么处理"],
          rows: [
            [
              "连接被拒绝",
              "服务没启动，或端口填成了 Web 界面的端口",
              `在「概览」确认运行中；SMTP 用 ${port}，不是 ${WEB_UI_PORT}`,
            ],
            [
              "报 STARTTLS 相关错误",
              "客户端强制要求加密",
              "关掉 starttls / secure，本地明文连接即可",
            ],
            [
              "报认证失败",
              "配了用户名密码",
              "清空 username 和 password，把 auth 设为 false",
            ],
            [
              "发送没报错但收件箱为空",
              "应用连的是别的 SMTP，或用了异步队列还没消费",
              "确认配置真的生效；检查邮件是否进了队列没发出",
            ],
            [
              "中文标题乱码",
              "没有指定字符集",
              "邮件头和正文都显式声明 UTF-8",
            ],
            [
              "邮件越积越多",
              "Mailpit 默认会保留历史邮件",
              "定期在界面上清空，或调用 DELETE 接口",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「邮件收件箱」标签页可以直接翻邮件、看 HTML 渲染效果和原始报文；「运行日志」能看到 SMTP 层面的连接记录，排查连不上的问题很有用；端口冲突时去「配置文件」标签页调整。",
        },
      ],
    },
  ];
}
