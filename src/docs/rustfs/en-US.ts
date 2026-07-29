import type { DocChapter } from "../docTypes";

export function buildRustfsDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Connect to RustFS",
      navHint: "S3 API · Console",
      title: "A local S3 object store written in Rust",
      intro:
        "RustFS exposes an Amazon S3-compatible API, ideal for debugging file uploads, buckets, presigned URLs, and object permissions.",
      blocks: [
        {
          kind: "text",
          value:
            "RustFS is an S3-compatible object store written from scratch in Rust. Like MinIO (implemented in Go), it ships both an S3 API and a web admin console, but the entire runtime is a single binary with a much smaller footprint. The macOS Apple Silicon build is still in Beta today—well suited to local development verification, and a great way to follow the Rust infrastructure ecosystem.",
        },
        {
          kind: "text",
          value: "Core capabilities of RustFS:",
        },
        {
          kind: "list",
          items: [
            "S3 API compatible: reuse existing AWS SDKs and S3 tooling without any code changes.",
            "Built-in web console: manage buckets, upload/download files, and inspect objects from the browser.",
            "Extremely low resource usage: a single Rust-compiled binary, typically consuming tens of MB of memory.",
            "Local file storage: data is written directly to the filesystem, with no extra storage engine dependency.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Description"],
          rows: [
            ["S3 Endpoint", `http://127.0.0.1:${port}`, "Application connection address"],
            ["Web Console", "http://127.0.0.1:7001", "Browser admin UI"],
            ["Access Key", "zhiyuadmin", "Local development account"],
            ["Secret Key", "zhiyu-local-rustfs-2026", "Local development password"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Beta release",
          value:
            "The official macOS Apple Silicon build is still Beta and only recommended for local development verification; the fixed dev credentials must not be used in production.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quickstart",
      navHint: "Bucket · Upload · Download",
      title: "Create a bucket and upload a file",
      intro:
        "RustFS is used almost identically to MinIO. Zhiyu's \"Connection & Console\" tab shows every connection detail, and the web console is directly reachable on port 7001.",
      blocks: [
        {
          kind: "list",
          items: [
            "Confirm the service shows \"Running\" on the Overview tab.",
            "The \"Connection & Console\" tab lists the S3 endpoint, web console URL, and credentials—copy them straight into your code.",
            "Open http://127.0.0.1:7001 in your browser and log in to the web console with the Access Key / Secret Key.",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Use mc (MinIO Client) with RustFS",
          code: `# Configure mc to point at RustFS
mc alias set rustfs http://127.0.0.1:${port} \\
  zhiyuadmin zhiyu-local-rustfs-2026

# Create a bucket
mc mb rustfs/uploads

# Upload a file
mc cp /path/to/file.txt rustfs/uploads/

# List files
mc ls rustfs/uploads/

# Download a file
mc cp rustfs/uploads/file.txt /tmp/

# Delete a file
mc rm rustfs/uploads/file.txt`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Call the S3 API directly with curl",
          code: `# List buckets
curl -s http://127.0.0.1:${port}/ \\
  -H "Authorization: AWS zhiyuadmin:..." \\
  | grep -o "<Name>[^<]*</Name>"

# Upload an object (PUT)
curl -X PUT http://127.0.0.1:${port}/uploads/hello.txt \\
  -H "Content-Type: text/plain" \\
  --data-binary "Hello from RustFS"

# Download an object (GET)
curl http://127.0.0.1:${port}/uploads/hello.txt

# Delete an object (DELETE)
curl -X DELETE http://127.0.0.1:${port}/uploads/hello.txt`,
        },
        {
          kind: "text",
          value:
            "The RustFS web console (port 7001) offers a UI very similar to the MinIO Console: create buckets, upload and download files, and inspect object metadata—all from the browser. It's a friendly entry point for developers who prefer to avoid the command line.",
        },
      ],
    },

    {
      id: "s3-compat",
      navLabel: "S3 compatibility",
      navHint: "SDK integration · common ops",
      title: "Reuse the entire S3 ecosystem",
      intro:
        "RustFS speaks the S3 API, so any code you wrote for MinIO or S3 can move to RustFS seamlessly—just point it at a different port.",
      blocks: [
        {
          kind: "text",
          value:
            "The common S3 operations below all work on RustFS, and the SDK code is identical to the MinIO chapter—only the port changes. See the \"Language integration\" chapter of the MinIO docs for per-language examples; only the differences are listed here:",
        },
        {
          kind: "table",
          head: ["Operation", "RustFS", "Difference vs. MinIO"],
          rows: [
            ["Create bucket", "Fully compatible", "No difference"],
            ["Upload object (PutObject)", "Fully compatible", "No difference"],
            ["Download object (GetObject)", "Fully compatible", "No difference"],
            ["Delete object (DeleteObject)", "Fully compatible", "No difference"],
            ["List objects (ListObjects)", "Fully compatible", "No difference"],
            ["Presigned URLs", "Partial support", "The Beta may not cover every presign type; prefer mc share"],
            ["Multipart upload", "Partial support", "Test large uploads locally first"],
            ["Bucket policy", "Partial support", "Permission features are still being fleshed out"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Switching is trivial",
          value:
            "If your code already reads the MinIO endpoint from an environment variable, moving to RustFS is just changing the port in MINIO_ENDPOINT. That interchangeability is exactly the value of the S3-compatible protocol.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language integration",
      navHint: "Java · Go · TS · Python",
      title: "Wire it into your project",
      intro: `RustFS integrates through the same SDKs as MinIO—both speak the S3 API. Below is a full Go example; for other languages, see the \"Language integration\" chapter of the MinIO docs and just swap the endpoint port to ${port}.`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Go",
              lang: "go",
              caption: "MinIO Go SDK against RustFS",
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
              caption: "MinIO JS SDK against RustFS",
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
              caption: "MinIO Python SDK against RustFS",
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
              caption: "AWS SDK v2 against RustFS",
              code: `// Key difference: swap the endpoint port to RustFS; everything else matches MinIO
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
          title: "Presigned URLs can be flaky in Beta",
          value:
            "Presigned URLs in the RustFS Beta may not cover every operation type. If a presigned URL fails to load, fall back to mc share to generate a temporary access link, or proxy the download through your application backend.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls & tuning",
      navHint: "Troubleshooting · common issues",
      title: "What to know before shipping",
      intro: "RustFS is still Beta—here are a few known caveats to keep in mind.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Workaround"],
          rows: [
            [
              "Some S3 APIs unavailable",
              "An S3 API call returns 501 Not Implemented",
              "Beta API coverage is incomplete; verify the operation first with mc or curl",
            ],
            [
              "Presigned URL fails",
              "Generated link is not reachable",
              "Beta limitation; use mc share to produce an access link instead",
            ],
            [
              "Large upload fails",
              "Files above a certain size fail to upload",
              "Try multipart upload or reduce per-file size",
            ],
            [
              "Incomplete permission features",
              "Setting a Bucket Policy has no effect",
              "Permission features are still evolving in Beta; manage through mc for now",
            ],
            [
              "Data incompatibility after upgrade",
              "Existing data cannot be read after upgrading RustFS",
              "Snapshot via \"Backup & restore\" before upgrading; watch the official release notes for data compatibility details",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to handle it in Zhiyu",
          value:
            "The Overview tab shows service status; open http://127.0.0.1:7001 in your browser to manage via the web console; the Runtime Logs surface startup errors and API call records; and always snapshot from \"Backup & restore\" before risky operations—especially on a Beta build.",
        },
      ],
    },
  ];
}
