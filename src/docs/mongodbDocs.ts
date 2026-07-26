import type { DocChapter } from "./docTypes";

/** MongoDB 使用文档。 */
export function buildMongodbDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 MongoDB",
      navHint: "文档模型 · 何时该用",
      title: "MongoDB 是什么",
      intro:
        "MongoDB 是一个面向文档的数据库。它不用「表和行」，而是用「集合和文档」——每个文档就是一份类似 JSON 的结构化数据，同一个集合里的文档甚至可以有不同的字段。",
      blocks: [
        {
          kind: "text",
          value:
            "关系型数据库要求你先定义好表结构，之后每一行都必须严格遵守。MongoDB 反过来：结构写在数据里，加字段不需要改表，特别适合需求还在变、或者不同记录天然就长得不一样的场景。",
        },
        {
          kind: "table",
          head: ["关系型数据库", "MongoDB", "说明"],
          rows: [
            ["数据库", "数据库", "概念一致"],
            ["表 table", "集合 collection", "文档的容器，无需预定义结构"],
            ["行 row", "文档 document", "一份 BSON 数据，形如 JSON"],
            ["列 column", "字段 field", "文档里的键"],
            ["主键", "_id", "不指定时自动生成 ObjectId"],
            ["JOIN", "内嵌文档 / $lookup", "优先内嵌，关联查询能力较弱"],
          ],
        },
        {
          kind: "text",
          value: "适合用它的情况：",
        },
        {
          kind: "list",
          items: [
            "字段经常变动，或每条记录的字段差异很大，比如商品属性、埋点事件、第三方回调。",
            "数据天然有层级，比如一篇文章带着它的标签、附件、评论，用内嵌文档一次读出来最省事。",
            "写入量大且以按主键或索引读取为主，不太需要复杂多表关联。",
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "需要强一致的核心账务仍建议用关系库",
          value:
            "MongoDB 现在支持多文档事务，但代价比关系库高，而且它的强项本来就不在复杂关联和约束上。涉及资金、库存对账这类场景，用 MySQL 或 PostgreSQL 会更稳妥。",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "「无 schema」不等于「不用设计」",
          value:
            "不预定义结构只是把约束从数据库挪到了应用层。如果没人管，同一个集合里很快会出现 userId、user_id、uid 三种写法。建议在应用层用类型定义约束，或者直接开启 MongoDB 的 schema 校验规则。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "连接 · 第一条文档",
      title: "连上并写入第一份文档",
      intro: "智屿已经把 MongoDB 装好并托管在本地，直接连就行。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "「数据浏览」标签页可以按库、按集合翻文档，不用手写查询。",
            "「JSON 命令台」标签页可以直接执行命令，下面的例子都能在那里跑。",
          ],
        },
        {
          kind: "table",
          head: ["参数", "值", "说明"],
          rows: [
            ["主机", "127.0.0.1", "只监听本机"],
            ["端口", String(port), "可在「配置文件」标签页修改"],
            ["认证", "无", "本地开发实例默认不开启认证"],
            ["连接串", `mongodb://127.0.0.1:${port}/demo`, "各语言驱动通用格式"],
          ],
        },
        {
          kind: "text",
          value:
            "集合不需要提前创建，第一次写入时会自动建出来。注意文档可以直接内嵌对象和数组：",
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "写入文档",
          code: `db.users.insertOne({
  name: "张三",
  email: "zhang@demo.com",
  age: 28,
  tags: ["后端", "Go"],
  profile: { city: "杭州", vip: true },   // 直接内嵌一个对象
  createdAt: new Date()
})

db.users.insertMany([
  { name: "李四", email: "li@demo.com", age: 31, tags: ["前端"] },
  { name: "王五", email: "wang@demo.com", age: 25 }   // 字段可以不一致
])`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "_id 是自动生成的",
          value:
            "不指定 _id 时，MongoDB 会生成一个 12 字节的 ObjectId，它内部含有时间戳，所以按 _id 排序大致等于按插入时间排序。需要用业务主键时也可以自己指定 _id，但必须保证唯一。",
        },
      ],
    },

    {
      id: "crud",
      navLabel: "查询与更新",
      navHint: "条件 · 操作符",
      title: "文档的读写",
      intro:
        "MongoDB 的查询条件本身就是一个文档，用各种以 $ 开头的操作符来表达比较和逻辑关系。",
      blocks: [
        {
          kind: "code",
          lang: "javascript",
          caption: "查询",
          code: `db.users.find({ age: 28 })                    // 等值
db.users.findOne({ email: "zhang@demo.com" }) // 只取一条

db.users.find({ age: { $gte: 25, $lt: 40 } }) // 范围
db.users.find({ age: { $in: [25, 28, 31] } }) // 枚举
db.users.find({ age: { $ne: 28 } })           // 不等于

// 多条件默认是 AND；OR 要显式写
db.users.find({ age: { $gte: 25 }, "profile.city": "杭州" })
db.users.find({ $or: [{ age: { $lt: 20 } }, { "profile.vip": true }] })

// 数组：只要包含该元素即可命中
db.users.find({ tags: "后端" })
db.users.find({ tags: { $all: ["后端", "Go"] } })   // 必须同时包含

// 字段是否存在
db.users.find({ phone: { $exists: false } })`,
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "投影、排序、分页",
          code: `// 第二个参数是投影：1 表示返回，0 表示排除
db.users.find({ age: { $gte: 25 } }, { name: 1, email: 1, _id: 0 })

db.users.find()
  .sort({ createdAt: -1 })   // -1 倒序，1 正序
  .skip(20)
  .limit(10)

db.users.countDocuments({ age: { $gte: 25 } })`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "更新一定要带更新操作符",
          value:
            "写 updateOne(filter, { name: '张三' }) 会报错或整体替换文档，把其他字段全丢掉。正确写法是 { $set: { name: '张三' } }，只改指定字段。",
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "更新",
          code: `db.users.updateOne(
  { email: "zhang@demo.com" },
  { $set: { age: 29, "profile.city": "上海" } }   // 支持按路径改嵌套字段
)

db.users.updateMany({ age: { $lt: 18 } }, { $set: { status: "minor" } })

db.users.updateOne({ _id: id }, { $inc: { loginCount: 1 } })   // 原子自增
db.users.updateOne({ _id: id }, { $unset: { phone: "" } })     // 删除字段

// 数组操作
db.users.updateOne({ _id: id }, { $push:     { tags: "缓存" } })  // 追加
db.users.updateOne({ _id: id }, { $addToSet: { tags: "缓存" } })  // 追加且去重
db.users.updateOne({ _id: id }, { $pull:     { tags: "前端" } })  // 移除

// upsert：查不到就插入一条
db.users.updateOne(
  { email: "new@demo.com" },
  { $set: { name: "新用户", age: 20 } },
  { upsert: true }
)`,
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "删除",
          code: `db.users.deleteOne({ _id: id })
db.users.deleteMany({ status: "minor" })`,
        },
      ],
    },

    {
      id: "aggregate",
      navLabel: "聚合与索引",
      navHint: "管道 · 建索引",
      title: "聚合管道和索引",
      intro:
        "聚合管道是 MongoDB 做统计分析的方式：把文档流依次经过多个阶段处理，每个阶段的输出是下个阶段的输入，相当于 SQL 的 GROUP BY 加上更强的变换能力。",
      blocks: [
        {
          kind: "code",
          lang: "javascript",
          caption: "聚合管道",
          code: `db.orders.aggregate([
  // 1. 先过滤，尽量放在最前面以减少后续处理量
  { $match: { status: "paid" } },

  // 2. 按用户分组统计
  { $group: {
      _id: "$userId",                  // 分组键，用 $字段名 引用
      orderCount: { $sum: 1 },
      totalAmount: { $sum: "$amount" },
      avgAmount:   { $avg: "$amount" }
  }},

  // 3. 排序取前 10
  { $sort: { totalAmount: -1 } },
  { $limit: 10 },

  // 4. 整理输出字段
  { $project: { _id: 0, userId: "$_id", orderCount: 1, totalAmount: 1 } }
])`,
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "关联与展开",
          code: `db.orders.aggregate([
  // $lookup 相当于 LEFT JOIN，把 users 里匹配的文档放进 user 数组
  { $lookup: {
      from: "users",
      localField: "userId",
      foreignField: "_id",
      as: "user"
  }},
  // $unwind 把数组摊平成多条文档，这里等价于取出唯一的那个用户
  { $unwind: "$user" },
  { $project: { amount: 1, status: 1, userName: "$user.name" } }
])`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "$lookup 比想象中贵",
          value:
            "它没有关系库那样成熟的连接优化，数据量大时很慢。MongoDB 的推荐做法是通过合理建模避免关联——把经常一起读取的数据直接内嵌进同一个文档。",
        },
        {
          kind: "text",
          value:
            "索引的作用和关系库一样：没有索引的查询要扫描整个集合。查询条件、排序字段都应该被索引覆盖：",
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "索引",
          code: `db.users.createIndex({ email: 1 }, { unique: true })   // 唯一索引
db.orders.createIndex({ userId: 1, status: 1 })        // 联合索引，顺序有讲究
db.users.createIndex({ "profile.city": 1 })            // 嵌套字段
db.users.createIndex({ tags: 1 })                      // 数组字段

// TTL 索引：文档在指定时间后自动删除，适合会话、临时数据
db.sessions.createIndex({ createdAt: 1 }, { expireAfterSeconds: 3600 })

db.users.getIndexes()
db.users.dropIndex("email_1")

// 用 explain 确认是否走了索引
db.users.find({ email: "zhang@demo.com" }).explain("executionStats")
// 看 winningPlan.stage：IXSCAN 表示走了索引，COLLSCAN 表示全集合扫描`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "内嵌还是引用？",
          value:
            "一对少、且总是一起读取的数据（文章和它的标签）用内嵌；一对多且数量可能无限增长的（用户和他的订单）用引用存 id。判断标准是单个文档不要超过 16MB，也不要出现无边界增长的数组。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 127.0.0.1:${port}。共同要点：客户端内部自带连接池，全局复用一个实例即可，不要每次请求都新建。`,
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
              caption: "MongoTemplate 读写",
              code: `@Document(collection = "users")
public class User {
    @Id
    private String id;
    private String name;
    private String email;
    private Integer age;
    // getter / setter 省略
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
              caption: "安装驱动",
              code: `go get go.mongodb.org/mongo-driver/mongo`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "官方驱动读写",
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

    // Client 内部自带连接池，全局复用
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
              caption: "安装驱动",
              code: `npm install mongodb`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "官方驱动读写",
              code: `import { MongoClient, type Collection } from "mongodb";

interface User {
  name: string;
  email: string;
  age: number;
  createdAt: Date;
}

// 模块级单例，客户端自带连接池
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
              caption: "安装驱动",
              code: `pip install pymongo`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "PyMongo 读写",
              code: `from datetime import datetime, timezone

from pymongo import MongoClient, DESCENDING

# 模块级单例，MongoClient 自带连接池
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
      navLabel: "避坑与调优",
      navHint: "建模 · 排查",
      title: "上线前该知道的事",
      intro: "MongoDB 的坑大多来自建模和索引，而不是数据库本身。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "全集合扫描",
              "查询随数据量线性变慢",
              "用 explain 确认 stage，给查询和排序字段建索引",
            ],
            [
              "无边界数组",
              "文档越来越大，最终超过 16MB 上限",
              "改成独立集合用引用关联，不要把评论、日志无限内嵌",
            ],
            [
              "字段命名不一致",
              "同一集合里 userId / user_id 混用",
              "应用层用类型约束，或开启 schema 校验",
            ],
            [
              "skip 深分页慢",
              "skip 值很大时越翻越慢",
              "改用「上次最后一个 _id」作为游标继续查",
            ],
            [
              "默认写关注不够",
              "极端情况下写入确认后仍可能丢",
              "关键写入显式指定 writeConcern 为 majority",
            ],
          ],
        },
        {
          kind: "code",
          lang: "javascript",
          caption: "排查命令",
          code: `db.serverStatus().connections      // 连接数使用情况
db.stats()                         // 当前库的数据量和索引大小
db.users.stats()                   // 单个集合的统计
db.currentOp()                     // 正在执行的操作，找卡住的查询
db.killOp(opid)                    // 终止某个操作

// 确认查询是否走索引
db.users.find({ email: "a@demo.com" }).explain("executionStats")`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「数据浏览」标签页可以直接翻集合和文档，不用手写查询；「JSON 命令台」适合执行上面这些命令；「运行日志」能看到启动报错；做危险操作前建议先去「备份恢复」标签页打一个快照。",
        },
      ],
    },
  ];
}
