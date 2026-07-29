import type { DocChapter } from "../docTypes";

export function buildMinioDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Connect to MinIO",
      navHint: "S3 API · Console",
      title: "Local S3-compatible object storage",
      intro:
        "MinIO exposes an Amazon S3-compatible API for local apps — ideal for debugging file uploads, buckets, presigned URLs, and object permissions.",
      blocks: [
        {
          kind: "text",
          value:
            "MinIO is the most widely used open-source object storage today. It is fully compatible with the AWS S3 API, meaning you can connect the AWS SDK straight to it and every tool in the S3 ecosystem (awscli, s3cmd, MinIO Client) works out of the box. For local development it is the best way to simulate S3 — no AWS account required, no charges, no network limits.",
        },
        {
          kind: "text",
          value: "What MinIO is good at:",
        },
        {
          kind: "list",
          items: [
            "Object storage: store files, images, video, logs and other unstructured data, up to 5 TiB per object.",
            "Presigned URLs: generate a time-limited link so users can upload or download directly from the browser without traffic going through the app server.",
            "Bucket policies: per-bucket permissions such as public-read, private, or restrict access to specific IPs.",
            "Event notifications: object upload/delete events can be published to Redis, NATS, or a webhook.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Notes"],
          rows: [
            ["S3 Endpoint", `http://127.0.0.1:${port}`, "Application endpoint"],
            ["Web Console", "http://127.0.0.1:9001", "Browser admin UI"],
            ["Access Key", "zhiyuadmin", "Local dev account"],
            ["Secret Key", "zhiyu-local-minio-2026", "Local dev password"],
            ["Data directory", "~/.devbox/instances/minio/default/data", "Object storage files"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Local development only",
          value:
            "These fixed dev credentials must not be used in production. The MinIO community repository has been archived; Zhiyu keeps it around to verify compatibility with existing projects. For production, use the official MinIO release or a cloud vendor's S3-compatible service.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Bucket · Upload · Download",
      title: "Create a bucket and upload a file",
      intro:
        "Zhiyu already has MinIO installed and running. The \"Connection & Console\" tab shows every connection detail and credential, and the browser console is available on port 9001.",
      blocks: [
        {
          kind: "list",
          items: [
            "Confirm the service is \"Running\" on the Overview tab.",
            "The \"Connection & Console\" tab shows the S3 endpoint, Web Console URL, and Access Key / Secret Key — copy them directly.",
            "Open http://127.0.0.1:9001 in the browser and sign in with the credentials to reach the graphical console, which supports bucket management and file operations.",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Using the MinIO Client (mc)",
          code: `# Install mc (the official MinIO command-line client)
brew install minio/stable/mc

# Configure the local MinIO connection
mc alias set local http://127.0.0.1:${port} \\
  zhiyuadmin zhiyu-local-minio-2026

# Create a bucket
mc mb local/uploads

# List buckets
mc ls local

# Upload a file
mc cp /path/to/photo.jpg local/uploads/

# List files
mc ls local/uploads/

# Download a file
mc cp local/uploads/photo.jpg /tmp/

# Delete a file
mc rm local/uploads/photo.jpg

# Remove a bucket (must be empty first)
mc rb local/uploads --force`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Using the AWS CLI (MinIO speaks the S3 protocol)",
          code: `# Install awscli
brew install awscli

# Configure (MinIO uses a custom endpoint)
aws configure set aws_access_key_id zhiyuadmin
aws configure set aws_secret_access_key zhiyu-local-minio-2026
aws configure set default.region us-east-1

# Specify the endpoint for each call
AWS_ENDPOINT=http://127.0.0.1:${port}

# Create a bucket
aws s3api create-bucket --bucket uploads \\
  --endpoint-url $AWS_ENDPOINT

# Upload
aws s3 cp photo.jpg s3://uploads/ \\
  --endpoint-url $AWS_ENDPOINT

# List objects
aws s3 ls s3://uploads/ \\
  --endpoint-url $AWS_ENDPOINT

# Download
aws s3 cp s3://uploads/photo.jpg /tmp/ \\
  --endpoint-url $AWS_ENDPOINT`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "mc fits MinIO better than awscli",
          value:
            "MinIO Client (mc) is developed by the MinIO project itself and pairs best with MinIO. It has unique features like tree-style directory browsing, recursive upload/download, and mirror sync. awscli works too, but you have to pass --endpoint-url every time, which is inconvenient.",
        },
      ],
    },

    {
      id: "operations",
      navLabel: "Common operations",
      navHint: "Presigned · Permissions",
      title: "A few of the most common scenarios",
      intro:
        "These are the S3 operations MinIO is most often used to simulate during local development, so the logic works unchanged once shipped.",
      blocks: [
        {
          kind: "text",
          value:
            "Presigned URLs are one of the most useful features: generate a time-limited URL and whoever holds it can upload or download files straight from the browser without going through the app backend. Use it during development to validate a frontend/backend split upload flow:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Generate presigned URLs",
          code: `# mc generates a download presigned URL (valid for 1 hour)
mc share download local/uploads/photo.jpg --expire 1h

# mc generates an upload presigned URL
mc share upload local/uploads/ --expire 30m

# Test the download link with curl
curl "<generated URL>" -o /tmp/photo.jpg`,
        },
        {
          kind: "text",
          value: "Set a bucket to public-read so its files can be accessed directly by URL:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Bucket permissions",
          code: `# Set public-read (anyone can download by URL)
mc policy set download local/uploads

# View the current policy
mc policy list local/uploads

# Restore to private
mc policy set private local/uploads`,
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Client integrations",
      navHint: "Java · Go · TS · Python",
      title: "Connect from your project",
      intro: `All snippets below point at the local http://127.0.0.1:${port}. Every language's AWS SDK is compatible with MinIO — just point the endpoint at it, turn off SSL, and use the fixed dev credentials.`,
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
              caption: "Upload and download",
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
    .forcePathStyle(true)  // MinIO must use path-style
    .build();

// Upload
s3.putObject(b -> b.bucket("uploads").key("photo.jpg"),
    RequestBody.fromFile(Paths.get("/path/to/photo.jpg")));

// Download
s3.getObject(b -> b.bucket("uploads").key("photo.jpg"),
    ResponseTransformer.toFile(Paths.get("/tmp/photo.jpg")));

// Generate a presigned download URL
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
              caption: "Install the minio-go SDK",
              code: `go get github.com/minio/minio-go/v7`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Upload and download",
              code: `package storage

import (
    "context"
    "time"

    "github.com/minio/minio-go/v7"
    "github.com/minio/minio-go/v7/pkg/credentials"
)

var client, _ = minio.New("127.0.0.1:${port}", &minio.Options{
    Creds:  credentials.NewStaticV4("zhiyuadmin", "zhiyu-local-minio-2026", ""),
    Secure: false, // Local MinIO uses HTTP
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

// Generate a presigned URL
func PresignedURL(ctx context.Context, bucket, object string) (string, error) {
    return client.PresignedGetObject(ctx, bucket, object, time.Hour, nil)
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "Install minio-js",
              code: `npm install minio`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Upload and download",
              code: `import * as Minio from "minio";

// Module-level singleton
const client = new Minio.Client({
  endPoint: "127.0.0.1",
  port: ${port},
  useSSL: false,
  accessKey: "zhiyuadmin",
  secretKey: "zhiyu-local-minio-2026",
});

// Ensure the bucket exists
async function ensureBucket(name: string) {
  const exists = await client.bucketExists(name);
  if (!exists) await client.makeBucket(name, "us-east-1");
}

// Upload a file
async function upload(bucket: string, object: string, filePath: string) {
  await client.fPutObject(bucket, object, filePath);
}

// Download a file
async function download(bucket: string, object: string, filePath: string) {
  await client.fGetObject(bucket, object, filePath);
}

// Generate a presigned download URL (valid for 1 hour)
async function presignedURL(bucket: string, object: string) {
  return client.presignedGetObject(bucket, object, 60 * 60);
}

// Upload from a stream
async function uploadStream(bucket: string, object: string, stream: ReadableStream) {
  await client.putObject(bucket, object, stream);
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install minio-py",
              code: `pip install minio`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Upload and download",
              code: `from minio import Minio
from minio.error import S3Error
from datetime import timedelta

client = Minio(
    "127.0.0.1:${port}",
    access_key="zhiyuadmin",
    secret_key="zhiyu-local-minio-2026",
    secure=False,
)

# Ensure the bucket exists
def ensure_bucket(name: str) -> None:
    if not client.bucket_exists(name):
        client.make_bucket(name, location="us-east-1")

# Upload a file
def upload(bucket: str, object_name: str, file_path: str) -> None:
    client.fput_object(bucket, object_name, file_path,
                       content_type="image/jpeg")

# Download a file
def download(bucket: str, object_name: str, file_path: str) -> None:
    client.fget_object(bucket, object_name, file_path)

# Generate a presigned download URL
def presigned_url(bucket: str, object_name: str) -> str:
    return client.presigned_get_object(bucket, object_name,
                                       expires=timedelta(hours=1))

# List objects in a bucket
def list_objects(bucket: str, prefix: str = "") -> list:
    return list(client.list_objects(bucket, prefix=prefix))`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "AWS Java SDK v2 must set forcePathStyle",
          value:
            "MinIO uses path-style URLs (the bucket is a path segment), whereas AWS SDK v2 defaults to virtual-hosted-style (the bucket is a subdomain). Without forcePathStyle(true) the MinIO connection will fail — a common gotcha. The official MinIO SDKs for Go and Python use path-style by default and need no extra setup.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls & tuning",
      navHint: "Troubleshooting · FAQ",
      title: "What to know before you ship",
      intro: "MinIO makes a perfect local stand-in for S3 during development, but a few things are worth watching.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Fix"],
          rows: [
            [
              "Connection failure: forcePathStyle",
              "Java SDK always errors when connecting to MinIO",
              "Set .forcePathStyle(true) when building the S3Client",
            ],
            [
              "SSL errors",
              "Client reports certificate errors",
              "Local MinIO runs over HTTP, so disable SSL (useSSL=false / secure=false)",
            ],
            [
              "Dashboard won't open",
              "Port 9001 shows a blank page in the browser",
              "Check that the service is running; older MinIO releases don't expose port 9001",
            ],
            [
              "Presigned URL unreachable",
              "The generated link doesn't work from outside",
              "The host inside a presigned URL is 127.0.0.1 and only works locally; set the SERVER_URL env var once MinIO is deployed",
            ],
            [
              "Large upload fails",
              "Files beyond a few hundred MB can't be uploaded",
              "Use multipart upload (supported by every SDK); a single PUT has a size limit",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do this inside Zhiyu",
          value:
            "The Overview tab shows service status; the \"Connection & Console\" tab shows every connection detail; open http://127.0.0.1:9001 to manage buckets and files from the graphical UI; the mc and awscli commands below run in a terminal; \"Runtime logs\" surface startup errors; and before any risky operation you can snapshot the data from \"Backup & restore\".",
        },
      ],
    },
  ];
}
