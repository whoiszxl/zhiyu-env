import type { DocChapter } from "../docTypes";

/** MongoDB usage documentation. */
export function buildMongodbDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet MongoDB",
      navHint: "Document model · When to use",
      title: "What is MongoDB",
      intro:
        "MongoDB is a document-oriented database. Instead of tables and rows, it uses collections and documents — each document is a JSON-like structured record, and documents in the same collection can even have different fields.",
      blocks: [
        {
          kind: "text",
          value:
            "Relational databases require you to define the table schema up front, and every row must adhere to it strictly. MongoDB flips this: the structure lives inside the data itself, adding a field doesn't require altering a table. It's especially well-suited when requirements are still evolving, or when different records naturally look different.",
        },
        {
          kind: "table",
          head: ["Relational database", "MongoDB", "Notes"],
          rows: [
            ["Database", "Database", "Same concept"],
            ["Table", "Collection", "Container for documents, no predefined structure"],
            ["Row", "Document", "A BSON record, shaped like JSON"],
            ["Column", "Field", "A key inside a document"],
            ["Primary key", "_id", "Auto-generated ObjectId when not specified"],
            ["JOIN", "Embedded document / $lookup", "Prefer embedding; join capabilities are limited"],
          ],
        },
        {
          kind: "text",
          value: "Good fits for MongoDB:",
        },
        {
          kind: "list",
          items: [
            "Fields change often, or records vary a lot from one to the next — product attributes, analytics events, third-party webhooks.",
            "Data is naturally hierarchical — an article with its tags, attachments, and comments; embedding lets you read it all in one shot.",
            "High write volume with reads mostly by primary key or index, without complex multi-table joins.",
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Still use a relational database for core, strongly-consistent bookkeeping",
          value:
            "MongoDB does support multi-document transactions now, but at a higher cost than a relational engine — and complex joins and constraints aren't its strong suit. For money, inventory reconciliation, and similar scenarios, MySQL or PostgreSQL is a safer choice.",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "\"Schemaless\" doesn't mean \"no design\"",
          value:
            "Skipping a predefined structure just shifts the constraint from the database to the application layer. Left unmanaged, the same collection will soon contain userId, user_id, and uid side by side. Enforce shape with types in your application, or turn on MongoDB's built-in schema validation.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Connect · First document",
      title: "Connect and write your first document",
      intro: "Zhiyu already installed and manages MongoDB locally — just connect.",
      blocks: [
        {
          kind: "list",
          items: [
            "Confirm the status shows \"Running\" on the Overview tab.",
            "The Data Explorer tab lets you browse documents by database and collection, no queries required.",
            "The JSON Console tab runs commands directly — all of the examples below work there.",
          ],
        },
        {
          kind: "table",
          head: ["Parameter", "Value", "Notes"],
          rows: [
            ["Host", "127.0.0.1", "Listens on the local machine only"],
            ["Port", String(port), "Can be changed on the Config File tab"],
            ["Auth", "None", "Local dev instances have auth disabled by default"],
            ["Connection string", `mongodb://127.0.0.1:${port}/demo`, "Standard format across drivers"],
          ],
        },
        {
          kind: "text",
          value:
            "Collections don't need to be created ahead of time — the first insert creates them automatically. Note that documents can embed objects and arrays directly:",
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Insert documents",
          code: `db.users.insertOne({
  name: "Zhang San",
  email: "zhang@demo.com",
  age: 28,
  tags: ["backend", "Go"],
  profile: { city: "Hangzhou", vip: true },   // embed an object directly
  createdAt: new Date()
})

db.users.insertMany([
  { name: "Li Si", email: "li@demo.com", age: 31, tags: ["frontend"] },
  { name: "Wang Wu", email: "wang@demo.com", age: 25 }   // fields can differ
])`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "_id is generated for you",
          value:
            "When you don't specify _id, MongoDB generates a 12-byte ObjectId containing a timestamp, so sorting by _id roughly matches insertion order. You can supply your own _id when you need a business key — just make sure it's unique.",
        },
      ],
    },

    {
      id: "crud",
      navLabel: "Query and update",
      navHint: "Filters · Operators",
      title: "Reading and writing documents",
      intro:
        "A MongoDB query is itself a document — comparisons and logic are expressed through operators that start with $.",
      blocks: [
        {
          kind: "code",
          lang: "javascript",
          caption: "Queries",
          code: `db.users.find({ age: 28 })                    // equality
db.users.findOne({ email: "zhang@demo.com" }) // single document

db.users.find({ age: { $gte: 25, $lt: 40 } }) // range
db.users.find({ age: { $in: [25, 28, 31] } }) // set membership
db.users.find({ age: { $ne: 28 } })           // not equal

// multiple conditions AND by default; OR must be explicit
db.users.find({ age: { $gte: 25 }, "profile.city": "Hangzhou" })
db.users.find({ $or: [{ age: { $lt: 20 } }, { "profile.vip": true }] })

// arrays: matches when the array contains the element
db.users.find({ tags: "backend" })
db.users.find({ tags: { $all: ["backend", "Go"] } })   // must contain both

// field existence
db.users.find({ phone: { $exists: false } })`,
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Projection, sort, and pagination",
          code: `// second argument is the projection: 1 to include, 0 to exclude
db.users.find({ age: { $gte: 25 } }, { name: 1, email: 1, _id: 0 })

db.users.find()
  .sort({ createdAt: -1 })   // -1 descending, 1 ascending
  .skip(20)
  .limit(10)

db.users.countDocuments({ age: { $gte: 25 } })`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Updates must use an update operator",
          value:
            "Writing updateOne(filter, { name: 'Zhang San' }) either errors or replaces the entire document, wiping out every other field. The correct form is { $set: { name: 'Zhang San' } }, which only touches the specified field.",
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Updates",
          code: `db.users.updateOne(
  { email: "zhang@demo.com" },
  { $set: { age: 29, "profile.city": "Shanghai" } }   // path syntax updates nested fields
)

db.users.updateMany({ age: { $lt: 18 } }, { $set: { status: "minor" } })

db.users.updateOne({ _id: id }, { $inc: { loginCount: 1 } })   // atomic increment
db.users.updateOne({ _id: id }, { $unset: { phone: "" } })     // remove a field

// array operations
db.users.updateOne({ _id: id }, { $push:     { tags: "cache" } })  // append
db.users.updateOne({ _id: id }, { $addToSet: { tags: "cache" } })  // append if absent
db.users.updateOne({ _id: id }, { $pull:     { tags: "frontend" } })  // remove

// upsert: insert if no document matches
db.users.updateOne(
  { email: "new@demo.com" },
  { $set: { name: "New user", age: 20 } },
  { upsert: true }
)`,
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Deletes",
          code: `db.users.deleteOne({ _id: id })
db.users.deleteMany({ status: "minor" })`,
        },
      ],
    },

    {
      id: "aggregate",
      navLabel: "Aggregation and indexes",
      navHint: "Pipeline · Indexing",
      title: "Aggregation pipelines and indexes",
      intro:
        "Aggregation pipelines are how MongoDB does analytics: a stream of documents flows through a sequence of stages, where each stage's output feeds the next — think SQL GROUP BY plus far more powerful transformations.",
      blocks: [
        {
          kind: "code",
          lang: "javascript",
          caption: "Aggregation pipeline",
          code: `db.orders.aggregate([
  // 1. filter first, keep it up front to reduce downstream work
  { $match: { status: "paid" } },

  // 2. group by user and aggregate
  { $group: {
      _id: "$userId",                  // group key, referenced with $fieldName
      orderCount: { $sum: 1 },
      totalAmount: { $sum: "$amount" },
      avgAmount:   { $avg: "$amount" }
  }},

  // 3. sort and take top 10
  { $sort: { totalAmount: -1 } },
  { $limit: 10 },

  // 4. shape the output fields
  { $project: { _id: 0, userId: "$_id", orderCount: 1, totalAmount: 1 } }
])`,
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Joining and unwinding",
          code: `db.orders.aggregate([
  // $lookup is like LEFT JOIN, placing matched users into the "user" array
  { $lookup: {
      from: "users",
      localField: "userId",
      foreignField: "_id",
      as: "user"
  }},
  // $unwind flattens the array into multiple documents; here it extracts the single matched user
  { $unwind: "$user" },
  { $project: { amount: 1, status: 1, userName: "$user.name" } }
])`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "$lookup is pricier than you think",
          value:
            "It lacks the mature join optimization of a relational engine and gets slow on large datasets. MongoDB's recommended approach is to model data so joins aren't needed — embed data that's read together into the same document.",
        },
        {
          kind: "text",
          value:
            "Indexes work the same as in relational databases: a query without a suitable index scans the entire collection. Filters and sort keys should both be covered by an index:",
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Indexes",
          code: `db.users.createIndex({ email: 1 }, { unique: true })   // unique index
db.orders.createIndex({ userId: 1, status: 1 })        // compound index — order matters
db.users.createIndex({ "profile.city": 1 })            // nested field
db.users.createIndex({ tags: 1 })                      // array field

// TTL index: documents auto-delete after the given time, great for sessions and temp data
db.sessions.createIndex({ createdAt: 1 }, { expireAfterSeconds: 3600 })

db.users.getIndexes()
db.users.dropIndex("email_1")

// use explain to confirm an index was used
db.users.find({ email: "zhang@demo.com" }).explain("executionStats")
// check winningPlan.stage: IXSCAN means the index was used, COLLSCAN means a full collection scan`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Embed or reference?",
          value:
            "Embed for one-to-few relationships that are always read together (an article and its tags). Use a reference (store the id) for one-to-many relationships that can grow without bound (a user and their orders). The rules of thumb: keep any single document under 16MB, and never let an array grow unboundedly.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language clients",
      navHint: "Java · Go · TS · Python",
      title: "Connect from your project",
      intro: `The configs below all point at 127.0.0.1:${port}. Common rule: the client has a built-in connection pool — reuse a single instance globally instead of creating a new one per request.`,
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
  <artifactId>spring-boot-starter-data-mongodb</artifactId>
</dependency>`,
            },
            {
              label: "Java",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  data:
    mongodb:
      uri: mongodb://127.0.0.1:${port}/demo`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Read and write with MongoTemplate",
              code: `@Document(collection = "users")
public class User {
    @Id
    private String id;
    private String name;
    private String email;
    private Integer age;
    // getters / setters omitted
}

@Repository
public class UserRepository {

    private final MongoTemplate mongo;

    public UserRepository(MongoTemplate mongo) {
        this.mongo = mongo;
    }

    public void insert(User user) {
        mongo.insert(user);
    }

    public List<User> findByMinAge(int minAge) {
        Query query = new Query(Criteria.where("age").gte(minAge))
                .with(Sort.by(Sort.Direction.DESC, "createdAt"))
                .limit(20);
        return mongo.find(query, User.class);
    }

    public void updateAge(String id, int age) {
        mongo.updateFirst(
            new Query(Criteria.where("_id").is(id)),
            new Update().set("age", age),
            User.class);
    }
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "Install the driver",
              code: `go get go.mongodb.org/mongo-driver/mongo`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Read and write with the official driver",
              code: `package store

import (
    "context"
    "time"

    "go.mongodb.org/mongo-driver/bson"
    "go.mongodb.org/mongo-driver/mongo"
    "go.mongodb.org/mongo-driver/mongo/options"
)

var users *mongo.Collection

func Init(ctx context.Context) error {
    opts := options.Client().
        ApplyURI("mongodb://127.0.0.1:${port}").
        SetMaxPoolSize(20).
        SetConnectTimeout(3 * time.Second)

    // Client has a built-in connection pool, reuse it globally
    client, err := mongo.Connect(ctx, opts)
    if err != nil {
        return err
    }
    users = client.Database("demo").Collection("users")
    return client.Ping(ctx, nil)
}

func Insert(ctx context.Context, name, email string, age int) error {
    _, err := users.InsertOne(ctx, bson.M{
        "name":      name,
        "email":     email,
        "age":       age,
        "createdAt": time.Now(),
    })
    return err
}

func FindByMinAge(ctx context.Context, minAge int) ([]User, error) {
    filter := bson.M{"age": bson.M{"$gte": minAge}}
    opts := options.Find().SetSort(bson.M{"createdAt": -1}).SetLimit(20)

    cursor, err := users.Find(ctx, filter, opts)
    if err != nil {
        return nil, err
    }
    defer cursor.Close(ctx)

    var result []User
    err = cursor.All(ctx, &result)
    return result, err
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "Install the driver",
              code: `npm install mongodb`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Read and write with the official driver",
              code: `import { MongoClient, type Collection } from "mongodb";

interface User {
  name: string;
  email: string;
  age: number;
  createdAt: Date;
}

// Module-level singleton; the client has a built-in connection pool
const client = new MongoClient("mongodb://127.0.0.1:${port}", {
  maxPoolSize: 20,
  connectTimeoutMS: 3000,
});

export const users: Collection<User> = client.db("demo").collection("users");

export async function connect(): Promise<void> {
  await client.connect();
}

export async function insertUser(name: string, email: string, age: number) {
  await users.insertOne({ name, email, age, createdAt: new Date() });
}

export async function findByMinAge(minAge: number): Promise<User[]> {
  return users
    .find({ age: { $gte: minAge } })
    .sort({ createdAt: -1 })
    .limit(20)
    .toArray();
}

export async function updateAge(email: string, age: number) {
  await users.updateOne({ email }, { $set: { age } });
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install the driver",
              code: `pip install pymongo`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Read and write with PyMongo",
              code: `from datetime import datetime, timezone

from pymongo import MongoClient, DESCENDING

# Module-level singleton; MongoClient has a built-in connection pool
client = MongoClient(
    "mongodb://127.0.0.1:${port}",
    maxPoolSize=20,
    connectTimeoutMS=3000,
)
users = client["demo"]["users"]


def insert_user(name: str, email: str, age: int) -> None:
    users.insert_one({
        "name": name,
        "email": email,
        "age": age,
        "createdAt": datetime.now(timezone.utc),
    })


def find_by_min_age(min_age: int) -> list[dict]:
    cursor = (
        users.find({"age": {"$gte": min_age}})
        .sort("createdAt", DESCENDING)
        .limit(20)
    )
    return list(cursor)


def update_age(email: str, age: int) -> None:
    users.update_one({"email": email}, {"$set": {"age": age}})`,
            },
          ],
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls and tuning",
      navHint: "Modeling · Diagnosis",
      title: "What to know before going live",
      intro: "Most MongoDB pitfalls come from modeling and indexing, not the database itself.",
      blocks: [
        {
          kind: "table",
          head: ["Problem", "Symptom", "Fix"],
          rows: [
            [
              "Full collection scan",
              "Queries slow down linearly with data size",
              "Use explain to check the stage, index filter and sort fields",
            ],
            [
              "Unbounded arrays",
              "Documents keep growing until they hit the 16MB limit",
              "Move into a separate collection with references — don't embed comments or logs without limit",
            ],
            [
              "Inconsistent field naming",
              "userId / user_id mixed in the same collection",
              "Enforce shape with types in your application, or enable schema validation",
            ],
            [
              "Slow deep pagination via skip",
              "The higher skip goes, the slower each page gets",
              "Use the last _id from the previous page as a cursor and keep going",
            ],
            [
              "Default write concern is too weak",
              "In edge cases writes can still be lost after being acknowledged",
              "Explicitly set writeConcern to majority for critical writes",
            ],
          ],
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "Diagnostic commands",
          code: `db.serverStatus().connections      // connection usage
db.stats()                         // data and index size for the current database
db.users.stats()                   // stats for a single collection
db.currentOp()                     // in-flight operations — spot stuck queries
db.killOp(opid)                    // terminate an operation

// verify a query uses an index
db.users.find({ email: "a@demo.com" }).explain("executionStats")`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do this in Zhiyu",
          value:
            "The Data Explorer tab lets you browse collections and documents without writing queries; the JSON Console is the right place to run the commands above; Runtime Logs surface startup errors; before any risky operation, take a snapshot from the Backup & Restore tab first.",
        },
      ],
    },
  ];
}
