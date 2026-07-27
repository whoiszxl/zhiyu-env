import type { DocChapter } from "./docTypes";

export function buildMinioDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 MinIO",
      navHint: "S3 API · Console",
      title: "本地 S3 兼容对象存储",
      intro:
        "MinIO 为本地应用提供兼容 Amazon S3 的 API，适合调试文件上传、Bucket、预签名 URL 和对象权限。",
      blocks: [
        {
          kind: "text",
          value:
            "MinIO 是目前使用最广泛的开源对象存储。它完全兼容 AWS S3 的 API，这意味着你能用 AWS SDK 直接连上它，所有 S3 生态的工具（awscli、s3cmd、MinIO Client）也都能无缝使用。本地开发时，它是模拟 S3 的最佳选择——不需要申请 AWS 账号、不产生费用、不受网络限制。",
        },
        {
          kind: "text",
          value: "MinIO 擅长做的事情：",
        },
        {
          kind: "list",
          items: [
            "对象存储：存文件、图片、视频、日志等非结构化数据，每个对象最大 5 TiB。",
            "预签名 URL：生成一个带时效的临时链接，让用户直接从浏览器上传或下载，流量不经过应用服务器。",
            "Bucket 策略：按桶设置公开读、私有、只允许特定 IP 访问等权限。",
            "事件通知：文件上传、删除等事件可以发布到 Redis、NATS 或 Webhook。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["S3 Endpoint", `http://127.0.0.1:${port}`, "应用连接地址"],
            ["Web Console", "http://127.0.0.1:9001", "浏览器管理界面"],
            ["Access Key", "zhiyuadmin", "本地开发账号"],
            ["Secret Key", "zhiyu-local-minio-2026", "本地开发密码"],
            ["数据目录", "~/.devbox/instances/minio/default/data", "对象存储文件"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "仅用于本地开发",
          value:
            "固定开发凭证不能用于生产环境。MinIO 社区仓库已归档，智屿保留它用于验证存量项目兼容性。生产环境请使用官方 MinIO 或云厂商的 S3 兼容服务。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "Bucket · 上传 · 下载",
      title: "创建 Bucket 并上传一个文件",
      intro:
        "智屿已经把 MinIO 装好并启动。「连接与控制台」标签页展示了所有连接信息和凭证，浏览器 Console 在 9001 端口可以直接打开管理。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认服务是「运行中」。",
            "「连接与控制台」标签页展示了 S3 Endpoint、Web Console 地址和 Access Key / Secret Key，可直接复制。",
            "浏览器打开 http://127.0.0.1:9001，用凭证登录即可看到图形管理界面，支持 Bucket 管理和文件操作。",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "用 MinIO Client (mc) 操作",
          code: `# 安装 mc（MinIO 官方命令行客户端）
brew install minio/stable/mc

# 配置本地 MinIO 连接
mc alias set local http://127.0.0.1:${port} \\
  zhiyuadmin zhiyu-local-minio-2026

# 创建 Bucket
mc mb local/uploads

# 查看 Bucket 列表
mc ls local

# 上传文件
mc cp /path/to/photo.jpg local/uploads/

# 查看文件
mc ls local/uploads/

# 下载文件
mc cp local/uploads/photo.jpg /tmp/

# 删除文件
mc rm local/uploads/photo.jpg

# 删除 Bucket（需要先清空）
mc rb local/uploads --force`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "用 AWS CLI 操作（MinIO 兼容 S3 协议）",
          code: `# 安装 awscli
brew install awscli

# 配置（MinIO 用自定义 endpoint）
aws configure set aws_access_key_id zhiyuadmin
aws configure set aws_secret_access_key zhiyu-local-minio-2026
aws configure set default.region us-east-1

# 操作时指定 endpoint
AWS_ENDPOINT=http://127.0.0.1:${port}

# 创建 Bucket
aws s3api create-bucket --bucket uploads \\
  --endpoint-url $AWS_ENDPOINT

# 上传
aws s3 cp photo.jpg s3://uploads/ \\
  --endpoint-url $AWS_ENDPOINT

# 列出文件
aws s3 ls s3://uploads/ \\
  --endpoint-url $AWS_ENDPOINT

# 下载
aws s3 cp s3://uploads/photo.jpg /tmp/ \\
  --endpoint-url $AWS_ENDPOINT`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "mc 比 awscli 更适合 MinIO",
          value:
            "MinIO Client (mc) 是 MinIO 项目自己开发的命令行工具，和 MinIO 配合最好。它有树形目录浏览、递归上传下载、镜像同步等独有功能。awscli 也能用，但每次都要带 --endpoint-url，不够方便。",
        },
      ],
    },

    {
      id: "operations",
      navLabel: "常用操作",
      navHint: "预签名 · 权限",
      title: "几个最常用的场景",
      intro:
        "本地开发中 MinIO 最常用来模拟这些 S3 操作，提前验证逻辑上线后无需改动。",
      blocks: [
        {
          kind: "text",
          value:
            "预签名 URL 是最常用的功能之一：生成一个有时效的 URL，持有者可以直接从浏览器上传或下载文件，无需经过应用后端。开发阶段用它来验证前后端分离的文件上传流程：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "生成预签名 URL",
          code: `# mc 生成下载预签名 URL（有效期 1 小时）
mc share download local/uploads/photo.jpg --expire 1h

# mc 生成上传预签名 URL
mc share upload local/uploads/ --expire 30m

# 用 curl 测试下载链接
curl "<生成的URL>" -o /tmp/photo.jpg`,
        },
        {
          kind: "text",
          value: "设置 Bucket 为公开读，这样里面的文件可以直接通过 URL 访问：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Bucket 权限",
          code: `# 设为公开读（任何人可以直接通过 URL 下载）
mc policy set download local/uploads

# 查看当前策略
mc policy list local/uploads

# 恢复为私有
mc policy set private local/uploads`,
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的代码都指向本机 http://127.0.0.1:${port}。所有语言的 AWS SDK 都兼容 MinIO——只需把 endpoint 指过来、关闭 SSL、用固定开发凭证即可。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java (AWS SDK v2)",
              lang: "xml",
              caption: "pom.xml",
              code: `<dependency>
  <groupId>software.amazon.awssdk</groupId>
  <artifactId>s3</artifactId>
  <version>2.31.20</version>
</dependency>`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "上传与下载",
              code: `import software.amazon.awssdk.auth.credentials.*;
import software.amazon.awssdk.core.sync.*;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.s3.S3Client;
import software.amazon.awssdk.services.s3.presigner.S3Presigner;
import java.net.URI;
import java.nio.file.Paths;
import java.time.Duration;

S3Client s3 = S3Client.builder()
    .endpointOverride(URI.create("http://127.0.0.1:${port}"))
    .credentialsProvider(StaticCredentialsProvider.create(
        AwsBasicCredentials.create("zhiyuadmin", "zhiyu-local-minio-2026")))
    .region(Region.US_EAST_1)
    .forcePathStyle(true)  // MinIO 必须走 path-style
    .build();

// 上传
s3.putObject(b -> b.bucket("uploads").key("photo.jpg"),
    RequestBody.fromFile(Paths.get("/path/to/photo.jpg")));

// 下载
s3.getObject(b -> b.bucket("uploads").key("photo.jpg"),
    ResponseTransformer.toFile(Paths.get("/tmp/photo.jpg")));

// 生成预签名下载 URL
S3Presigner presigner = S3Presigner.builder()
    .endpointOverride(URI.create("http://127.0.0.1:${port}"))
    .credentialsProvider(StaticCredentialsProvider.create(
        AwsBasicCredentials.create("zhiyuadmin", "zhiyu-local-minio-2026")))
    .region(Region.US_EAST_1)
    .build();
var request = presigner.presignGetObject(b -> b
    .getObjectRequest(r -> r.bucket("uploads").key("photo.jpg"))
    .signatureDuration(Duration.ofHours(1)));
System.out.println(request.url());`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 minio-go SDK",
              code: `go get github.com/minio/minio-go/v7`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "上传与下载",
              code: `package storage

import (
    "context"
    "time"

    "github.com/minio/minio-go/v7"
    "github.com/minio/minio-go/v7/pkg/credentials"
)

var client, _ = minio.New("127.0.0.1:${port}", &minio.Options{
    Creds:  credentials.NewStaticV4("zhiyuadmin", "zhiyu-local-minio-2026", ""),
    Secure: false, // MinIO 本地用 HTTP
})

func EnsureBucket(ctx context.Context, name string) error {
    exists, err := client.BucketExists(ctx, name)
    if err != nil {
        return err
    }
    if !exists {
        return client.MakeBucket(ctx, name, minio.MakeBucketOptions{Region: "us-east-1"})
    }
    return nil
}

func UploadFile(ctx context.Context, bucket, object, filePath string) error {
    _, err := client.FPutObject(ctx, bucket, object, filePath,
        minio.PutObjectOptions{ContentType: "image/jpeg"})
    return err
}

func DownloadFile(ctx context.Context, bucket, object, filePath string) error {
    return client.FGetObject(ctx, bucket, object, filePath, minio.GetObjectOptions{})
}

// 生成预签名 URL
func PresignedURL(ctx context.Context, bucket, object string) (string, error) {
    return client.PresignedGetObject(ctx, bucket, object, time.Hour, nil)
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 minio-js",
              code: `npm install minio`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "上传与下载",
              code: `import * as Minio from "minio";

// 模块级单例
const client = new Minio.Client({
  endPoint: "127.0.0.1",
  port: ${port},
  useSSL: false,
  accessKey: "zhiyuadmin",
  secretKey: "zhiyu-local-minio-2026",
});

// 确保 Bucket 存在
async function ensureBucket(name: string) {
  const exists = await client.bucketExists(name);
  if (!exists) await client.makeBucket(name, "us-east-1");
}

// 上传文件
async function upload(bucket: string, object: string, filePath: string) {
  await client.fPutObject(bucket, object, filePath);
}

// 下载文件
async function download(bucket: string, object: string, filePath: string) {
  await client.fGetObject(bucket, object, filePath);
}

// 生成预签名下载 URL（1 小时有效）
async function presignedURL(bucket: string, object: string) {
  return client.presignedGetObject(bucket, object, 60 * 60);
}

// 上传文件流
async function uploadStream(bucket: string, object: string, stream: ReadableStream) {
  await client.putObject(bucket, object, stream);
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 minio-py",
              code: `pip install minio`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "上传与下载",
              code: `from minio import Minio
from minio.error import S3Error
from datetime import timedelta

client = Minio(
    "127.0.0.1:${port}",
    access_key="zhiyuadmin",
    secret_key="zhiyu-local-minio-2026",
    secure=False,
)

# 确保 Bucket 存在
def ensure_bucket(name: str) -> None:
    if not client.bucket_exists(name):
        client.make_bucket(name, location="us-east-1")

# 上传文件
def upload(bucket: str, object_name: str, file_path: str) -> None:
    client.fput_object(bucket, object_name, file_path,
                       content_type="image/jpeg")

# 下载文件
def download(bucket: str, object_name: str, file_path: str) -> None:
    client.fget_object(bucket, object_name, file_path)

# 生成预签名下载 URL
def presigned_url(bucket: str, object_name: str) -> str:
    return client.presigned_get_object(bucket, object_name,
                                       expires=timedelta(hours=1))

# 列出 Bucket 中的文件
def list_objects(bucket: str, prefix: str = "") -> list:
    return list(client.list_objects(bucket, prefix=prefix))`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "AWS Java SDK v2 必须设 forcePathStyle",
          value:
            "MinIO 使用 path-style URL（bucket 作为路径的一段），而 AWS SDK v2 默认使用 virtual-hosted-style（bucket 作为子域名）。不设 forcePathStyle(true) 会导致 MinIO 连接失败，这是一个常见的踩坑点。Go 和 Python 的 MinIO 官方 SDK 默认就是 path-style，不需要额外设置。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "MinIO 作为 S3 的本地替代在开发阶段很完美，但有几个点值得留意。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "连接失败：forcePathStyle",
              "Java SDK 连 MinIO 总是报错",
              "构建 S3Client 时设置 .forcePathStyle(true)",
            ],
            [
              "SSL 报错",
              "客户端提示证书错误",
              "本地 MinIO 走 HTTP，把 SSL 关掉（useSSL=false / secure=false）",
            ],
            [
              "Dashboard 打不开",
              "浏览器访问 9001 端口空白",
              "检查服务是否在运行中；如果用的是旧版 MinIO，9001 端口不存在",
            ],
            [
              "预签名 URL 无法访问",
              "生成的链接从外部访问不了",
              "预签名 URL 里的 host 是 127.0.0.1，只能本机访问；minio 上生产后需设置 SERVER_URL 环境变量",
            ],
            [
              "大文件上传失败",
              "超过几百 MB 的文件传不上去",
              "用分片上传（multipart upload），所有 SDK 都支持；单次 PUT 操作有大小限制",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页能看到服务状态；「连接与控制台」标签页展示了所有连接信息；打开 http://127.0.0.1:9001 可以用图形界面管理 Bucket 和文件；下面的 mc 和 awscli 命令在终端中运行；「运行日志」能看到启动报错；做危险操作前可以去「备份恢复」打一个数据快照。",
        },
      ],
    },
  ];
}
