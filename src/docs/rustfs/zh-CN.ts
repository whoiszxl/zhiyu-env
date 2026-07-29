import type { DocChapter } from "../docTypes";

export function buildRustfsDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 RustFS",
      navHint: "S3 API · Console",
      title: "Rust 实现的本地 S3 对象存储",
      intro:
        "RustFS 提供兼容 Amazon S3 的 API，可用于调试文件上传、Bucket、预签名 URL 和对象权限。",
      blocks: [
        {
          kind: "text",
          value:
            "RustFS 是一个用 Rust 从零实现的 S3 兼容对象存储。和 MinIO（Go 实现）一样，它提供 S3 API 和 Web 管理后台，但整个运行时只依赖一个二进制文件，资源占用更小。目前 macOS Apple Silicon 版本还处于 Beta 阶段，适合用于本地开发验证，也是跟踪 Rust 基础设施生态的绝佳选择。",
        },
        {
          kind: "text",
          value: "RustFS 的核心能力：",
        },
        {
          kind: "list",
          items: [
            "S3 API 兼容：沿用现有 AWS SDK 和 S3 工具链，无需切换代码。",
            "内置 Web Console：浏览器管理 Bucket、上传下载、查看对象。",
            "极低资源占用：Rust 编译的单个二进制，内存通常在数十 MB 级别。",
            "本地文件存储：数据直接写在文件系统，无额外存储引擎依赖。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["S3 Endpoint", `http://127.0.0.1:${port}`, "应用连接地址"],
            ["Web Console", "http://127.0.0.1:7001", "浏览器管理界面"],
            ["Access Key", "zhiyuadmin", "本地开发账号"],
            ["Secret Key", "zhiyu-local-rustfs-2026", "本地开发密码"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Beta 版本",
          value:
            "当前官方 macOS Apple Silicon 版本仍处于 Beta 阶段，仅建议用于本地开发验证；固定开发凭证不能用于生产环境。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "Bucket · 上传 · 下载",
      title: "创建 Bucket 并上传文件",
      intro:
        "RustFS 与 MinIO 用法高度一致。智屿的「连接与控制台」标签页展示了所有连接信息，Web Console 在 7001 端口可以直接打开。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认服务是「运行中」。",
            "「连接与控制台」标签页展示了 S3 Endpoint、Web Console 地址和凭证，可直接复制到代码里。",
            "浏览器打开 http://127.0.0.1:7001，用 Access Key / Secret Key 登录 Web Console。",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "用 mc（MinIO Client）操作 RustFS",
          code: `# 配置 mc 指向 RustFS
mc alias set rustfs http://127.0.0.1:${port} \\
  zhiyuadmin zhiyu-local-rustfs-2026

# 创建 Bucket
mc mb rustfs/uploads

# 上传文件
mc cp /path/to/file.txt rustfs/uploads/

# 查看文件列表
mc ls rustfs/uploads/

# 下载文件
mc cp rustfs/uploads/file.txt /tmp/

# 删除文件
mc rm rustfs/uploads/file.txt`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "用 curl 直接调 S3 API",
          code: `# 列出 Bucket
curl -s http://127.0.0.1:${port}/ \\
  -H "Authorization: AWS zhiyuadmin:..." \\
  | grep -o "<Name>[^<]*</Name>"

# 上传对象（PUT）
curl -X PUT http://127.0.0.1:${port}/uploads/hello.txt \\
  -H "Content-Type: text/plain" \\
  --data-binary "Hello from RustFS"

# 下载对象（GET）
curl http://127.0.0.1:${port}/uploads/hello.txt

# 删除对象（DELETE）
curl -X DELETE http://127.0.0.1:${port}/uploads/hello.txt`,
        },
        {
          kind: "text",
          value:
            "RustFS 的 Web Console（7001 端口）提供了和 MinIO Console 类似的图形界面：能在浏览器里创建 Bucket、上传下载文件、查看对象元数据。对于不习惯命令行的开发者来说，这是个很好的操作入口。",
        },
      ],
    },

    {
      id: "s3-compat",
      navLabel: "S3 兼容性",
      navHint: "SDK 接入 · common ops",
      title: "复用 S3 生态的全部工具",
      intro:
        "RustFS 兼容 S3 API，这意味着所有为 MinIO / S3 编写的代码可以无缝切换到 RustFS——改个端口就行。",
      blocks: [
        {
          kind: "text",
          value:
            "下面这些常见 S3 操作在 RustFS 上完全可用，对应的 SDK 代码和 MinIO 那一章一模一样，只需要把端口换成 RustFS 的。每个语言的接入示例，请参考 MinIO 文档的「语言接入」章节，这里只列出差异：",
        },
        {
          kind: "table",
          head: ["操作", "RustFS", "与 MinIO 差异"],
          rows: [
            ["创建 Bucket", "完全兼容", "无差异"],
            ["上传对象 (PutObject)", "完全兼容", "无差异"],
            ["下载对象 (GetObject)", "完全兼容", "无差异"],
            ["删除对象 (DeleteObject)", "完全兼容", "无差异"],
            ["列出对象 (ListObjects)", "完全兼容", "无差异"],
            ["预签名 URL", "部分支持", "Beta 版本可能不支持所有预签名类型；建议优先用 mc share"],
            ["分片上传", "部分支持", "大文件上传建议先本地测试确认"],
            ["Bucket Policy", "部分支持", "权限相关功能还在完善中"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "切换方式极其简单",
          value:
            "如果你已经把代码写成了「MinIO endpoint 从环境变量读取」，那么切换 RustFS 只需要改一下 MINIO_ENDPOINT 环境变量的端口号。这正是 S3 兼容协议的价值——实现可替换。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `RustFS 的 SDK 接入和 MinIO 完全一致，都兼容 S3 API。下面以 Go 为例展示完整接入代码，其他语言请参考 MinIO 文档的「语言接入」章节，把 endpoint 端口换成 ${port} 即可。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Go",
              lang: "go",
              caption: "MinIO Go SDK 连 RustFS",
              code: `package storage

import (
    "context"
    "time"

    "github.com/minio/minio-go/v7"
    "github.com/minio/minio-go/v7/pkg/credentials"
)

var client, _ = minio.New("127.0.0.1:${port}", &minio.Options{
    Creds:  credentials.NewStaticV4("zhiyuadmin", "zhiyu-local-rustfs-2026", ""),
    Secure: false,
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
        minio.PutObjectOptions{ContentType: "application/octet-stream"})
    return err
}

func DownloadFile(ctx context.Context, bucket, object, filePath string) error {
    return client.FGetObject(ctx, bucket, object, filePath, minio.GetObjectOptions{})
}

func PresignedURL(ctx context.Context, bucket, object string) (string, error) {
    return client.PresignedGetObject(ctx, bucket, object, time.Hour, nil)
}`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "MinIO JS SDK 连 RustFS",
              code: `import * as Minio from "minio";

const client = new Minio.Client({
  endPoint: "127.0.0.1",
  port: ${port},
  useSSL: false,
  accessKey: "zhiyuadmin",
  secretKey: "zhiyu-local-rustfs-2026",
});

async function upload(bucket: string, object: string, filePath: string) {
  await client.fPutObject(bucket, object, filePath);
}

async function download(bucket: string, object: string, filePath: string) {
  await client.fGetObject(bucket, object, filePath);
}`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "MinIO Python SDK 连 RustFS",
              code: `from minio import Minio
from datetime import timedelta

client = Minio(
    "127.0.0.1:${port}",
    access_key="zhiyuadmin",
    secret_key="zhiyu-local-rustfs-2026",
    secure=False,
)

def upload(bucket: str, object_name: str, file_path: str) -> None:
    client.fput_object(bucket, object_name, file_path)

def download(bucket: str, object_name: str, file_path: str) -> None:
    client.fget_object(bucket, object_name, file_path)

def presigned_url(bucket: str, object_name: str) -> str:
    return client.presigned_get_object(bucket, object_name,
                                       expires=timedelta(hours=1))`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "AWS SDK v2 连 RustFS",
              code: `// 关键差异：endpoint 端口换成 RustFS 的，其他和连 MinIO 一样
S3Client s3 = S3Client.builder()
    .endpointOverride(URI.create("http://127.0.0.1:${port}"))
    .credentialsProvider(StaticCredentialsProvider.create(
        AwsBasicCredentials.create("zhiyuadmin", "zhiyu-local-rustfs-2026")))
    .region(Region.US_EAST_1)
    .forcePathStyle(true)
    .build();

s3.putObject(b -> b.bucket("uploads").key("file.txt"),
    RequestBody.fromString("Hello RustFS"));`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Beta 版本的预签名 URL 可能不稳定",
          value:
            "预签名 URL 功能在 RustFS Beta 版本中不一定完全支持所有操作类型。如果遇到预签名 URL 无法访问的情况，可以回退到直接用 mc share 生成临时访问链接，或者通过应用后端中转下载。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "RustFS 还在 Beta，使用中有几个已知需要注意的点。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "部分 S3 API 不可用",
              "调用某个 S3 API 返回 501 Not Implemented",
              "Beta 版 API 覆盖不全；用 mc 或 curl 先验证该操作是否支持",
            ],
            [
              "预签名 URL 失败",
              "生成的链接无法访问",
              "Beta 限制；改用 mc share 生成访问链接",
            ],
            [
              "大文件上传失败",
              "超过特定大小的文件传不上去",
              "尝试分片上传或减小单文件体积",
            ],
            [
              "权限功能不完整",
              "设置 Bucket Policy 无效",
              "Beta 阶段权限功能在完善中；先用 mc 操作管理",
            ],
            [
              "升级后数据不兼容",
              "升级 RustFS 版本后原先数据读不了",
              "升级前务必用「备份恢复」打快照；关注官方 release notes 的数据兼容说明",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页能看到服务状态；浏览器打开 http://127.0.0.1:7001 可以用 Web Console 管理；「运行日志」能看到启动报错和 API 调用记录；做危险操作前记得去「备份恢复」打一个数据快照——Beta 版本尤其需要这个习惯。",
        },
      ],
    },
  ];
}
