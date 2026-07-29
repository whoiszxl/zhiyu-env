import type { DocChapter } from "../docTypes";

export function buildMeilisearchDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Meilisearch",
      navHint: "索引 · 全文搜索",
      title: "Meilisearch 是什么",
      intro:
        "Meilisearch 是面向应用开发的全文搜索引擎。把 JSON 文档写入索引后，即可通过 HTTP API 获得支持前缀和拼写容错的搜索结果。",
      blocks: [
        {
          kind: "text",
          value:
            "Meilisearch 和 Elasticsearch 解决的问题类似，但设计哲学完全不同。Elasticsearch 功能极其丰富但部署和调优成本高，Meilisearch 则追求「开箱即用」——一个二进制、零配置、毫秒级搜索、自带中文分词。绝大多数内部搜索、文档搜索、电商商品搜索用它就够了，不需要去折腾 Elasticsearch 那套 JVM 调优。",
        },
        {
          kind: "text",
          value: "它的核心能力：",
        },
        {
          kind: "list",
          items: [
            "前缀搜索与拼写容错：输入「zhnag」能搜到「张三」相关的文档。",
            "过滤器与排序：按价格、时间、分类等字段精确筛选和排序。",
            "分面聚合：搜索结果附带分类统计，可以用来做「按品牌/价格区间筛选」的 UI。",
            "异步索引：文档写入后异步建索引，写入接口秒级返回。",
            "多语言支持：内置中文、日文、韩文等 CJK 分词，无需额外插件。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "本地配置", "说明"],
          rows: [
            ["HTTP 地址", `http://127.0.0.1:${port}`, "应用和 SDK 连接地址"],
            ["环境", "development", "本地开发模式"],
            ["API Key", "未设置（dev 模式）", "切到 production 环境后必须设置"],
            ["分析数据", "关闭", "不会发送匿名遥测"],
            ["数据目录", "~/.devbox/instances/meilisearch/default/data", "索引文件"],
          ],
        },
        {
          kind: "table",
          head: ["", "Meilisearch", "Elasticsearch", "Algolia"],
          rows: [
            ["部署", "单个二进制", "Java 集群 + JVM 调优", "SaaS，无部署"],
            ["启动速度", "秒级", "分钟级", "无需启动"],
            ["中文分词", "内置", "需装 IK 等插件", "内置"],
            ["查询延迟", "毫秒级", "毫秒级（取决于索引规模）", "毫秒级"],
            ["配置复杂度", "极低，零配置可用", "高，需要大量调优", "中等"],
            ["适合场景", "中小规模应用内搜索", "海量日志、监控、全文检索", "对外搜索产品"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "索引类似一张用于搜索的表",
          value:
            "每个索引包含一组 JSON 文档，并用一个字段作为主键。文档字段默认都可以搜索，后续可通过 API 调整可搜索、可过滤和可排序字段。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "建索引 · 搜一下",
      title: "建一个索引然后搜",
      intro:
        "智屿已经把 Meilisearch 装好并启动。「索引与搜索」标签页集成了索引管理和搜索调试，不需要切到外部工具。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "「索引与搜索」标签页可以直接创建/选择索引、导入 JSON 文档和执行搜索，不用写任何代码。",
            "写代码时，下面的 curl 命令可以直接在终端或代码里运行。",
          ],
        },
        {
          kind: "table",
          head: ["参数", "值", "说明"],
          rows: [
            ["主机", "127.0.0.1", "只监听本机"],
            ["端口", String(port), "HTTP API 端口"],
            ["API Key", "（空，dev 模式）", "本地开发无需认证"],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "创建索引并写入文档",
          code: `BASE=http://127.0.0.1:${port}

# 创建索引（设置主键字段）
curl -X POST "$BASE/indexes" \\
  -H "Content-Type: application/json" \\
  -d '{"uid":"movies","primaryKey":"id"}'

# 写入一批文档
curl -X POST "$BASE/indexes/movies/documents" \\
  -H "Content-Type: application/json" \\
  -d '[
  {"id":1,"title":"盗梦空间","year":2010,"genres":["科幻","动作"],"rating":9.3},
  {"id":2,"title":"星际穿越","year":2014,"genres":["科幻","剧情"],"rating":9.4},
  {"id":3,"title":"泰坦尼克号","year":1997,"genres":["爱情","剧情"],"rating":9.5},
  {"id":4,"title":"千与千寻","year":2001,"genres":["动画","奇幻"],"rating":9.4},
  {"id":5,"title":"肖申克的救赎","year":1994,"genres":["剧情"],"rating":9.7}
]'

# 查看写入任务状态（taskUid 从上一步返回中获取）
curl "$BASE/indexes/movies/tasks"

# 等任务 status 变成 succeeded 后就可以搜了`,
        },
        {
          kind: "text",
          value: "文档写入后稍等一两秒等索引完成，然后搜一下：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "搜索",
          code: `# 基础搜索
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"星际"}'

# 搜索结果里：
#   hits          命中的文档列表
#   estimatedTotalHits  命中总数
#   processingTimeMs    查询耗时（毫秒）

# 带过滤器和排序的搜索
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{
    "q":"幻",
    "filter":"year >= 2001 AND rating > 9.0",
    "sort":["rating:desc"],
    "limit":5
  }'`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Search 端点是 POST，不是 GET",
          value:
            "Meilisearch 的搜索接口要求用 POST 方法传 JSON 参数，这一点和大多数 REST API 的习惯不一样。GET /indexes/movies/search?q=xxx 也能用，但参数多了之后 JSON Body 更清晰。",
        },
      ],
    },

    {
      id: "features",
      navLabel: "搜索能力",
      navHint: "过滤 · 排序 · 容错",
      title: "用好搜索的各种特性",
      intro:
        "Meilisearch 在零配置下已经能做不错的搜索，但把几个关键设置配好，体验会明显上一个台阶。",
      blocks: [
        {
          kind: "text",
          value:
            "Meilisearch 默认把文档的所有字段都设为「可搜索」的。但实际项目里，有些字段不需要搜索（比如 id、createdAt），可以用可搜索属性设置来限定范围，搜索精度会明显提升：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "限定可搜索字段",
          code: `curl -X PUT "$BASE/indexes/movies/settings/searchable-attributes" \\
  -H "Content-Type: application/json" \\
  -d '["title","genres"]'

# 查看当前设置
curl "$BASE/indexes/movies/settings/searchable-attributes"`,
        },
        {
          kind: "text",
          value:
            "同样，过滤和排序也需要显式声明哪些字段可以用来做这些操作。不声明的字段不能用于 filter 和 sort：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "设置可过滤和可排序字段",
          code: `curl -X PUT "$BASE/indexes/movies/settings/filterable-attributes" \\
  -H "Content-Type: application/json" \\
  -d '["year","rating","genres"]'

curl -X PUT "$BASE/indexes/movies/settings/sortable-attributes" \\
  -H "Content-Type: application/json" \\
  -d '["year","rating"]'

# 设置后才可以用：
#   filter: "year >= 2000 AND rating > 9.0"
#   sort: ["year:desc", "rating:asc"]

# 也支持布尔过滤
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"","filter":"genres IN [科幻, 动画]"}'`,
        },
        {
          kind: "text",
          value: "排序和分面聚合经常一起使用：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "排序与搜索示例",
          code: `# 按评分降序
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"电影","sort":["rating:desc"],"limit":10}'

# facets 获取分类统计（比如侧边栏筛选用）
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"","facets":["genres","year"]}'
# 返回的 facetDistribution 里会有 genres 和 year 的分布`,
        },
        {
          kind: "table",
          head: ["配置项", "作用", "建议"],
          rows: [
            ["searchableAttributes", "限定哪些字段参与全文搜索", "只放用户真正搜索的字段，排除 id 和内部字段"],
            ["filterableAttributes", "哪些字段可以用于 filter 条件", "放 year / rating / status 这类需要筛选的"],
            ["sortableAttributes", "哪些字段可以用于 sort 排序", "放数值和日期字段"],
            ["rankingRules", "调整相关性排序权重", "默认规则已经很好，不要轻易动"],
            ["typoTolerance", "拼写容错的开关和参数", "默认开启，中文场景一般不需要调整"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "每次改设置都会触发全量重新索引",
          value:
            "修改 searchableAttributes、filterableAttributes 等设置后，Meilisearch 会启动一个异步任务把所有文档重新索引一遍。数据量大时会花一些时间，但写入接口在此期间仍然可用。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 http://127.0.0.1:${port}。所有语言的 SDK 都是对 REST API 的封装，如果某个语言没有官方 SDK，直接调 HTTP 也可以。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "pom.xml",
              code: `<dependency>
  <groupId>com.meilisearch.sdk</groupId>
  <artifactId>meilisearch-java</artifactId>
  <version>0.13.5</version>
</dependency>`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "写入与搜索",
              code: `import com.meilisearch.sdk.*;
import com.google.gson.JsonObject;

Config config = new Config("http://127.0.0.1:${port}", null);
Client client = new Client(config);

// 写入文档
JsonObject doc = new JsonObject();
doc.addProperty("id", 1);
doc.addProperty("title", "盗梦空间");
doc.addProperty("year", 2010);
client.index("movies").addDocuments("[{...}]");

// 搜索
SearchRequest.SearchRequestBuilder builder = SearchRequest.builder()
    .q("星际")
    .filter(new String[]{"year >= 2000"})
    .sort(new String[]{"rating:desc"})
    .limit(10);
Results results = client.index("movies").search(builder.build());
System.out.println(results.getHits());`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 meilisearch-go",
              code: `go get github.com/meilisearch/meilisearch-go`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "写入与搜索",
              code: `package search

import "github.com/meilisearch/meilisearch-go"

var client = meilisearch.New("http://127.0.0.1:${port}")

type Movie struct {
    ID     int      \`json:"id"\`
    Title  string   \`json:"title"\`
    Year   int      \`json:"year"\`
    Genres []string \`json:"genres"\`
    Rating float64  \`json:"rating"\`
}

func AddMovies() error {
    movies := []Movie{
        {ID: 1, Title: "盗梦空间", Year: 2010, Genres: []string{"科幻", "动作"}, Rating: 9.3},
        {ID: 2, Title: "星际穿越", Year: 2014, Genres: []string{"科幻", "剧情"}, Rating: 9.4},
    }
    _, err := client.Index("movies").AddDocuments(movies, "id")
    return err
}

func Search(q string) (*meilisearch.SearchResponse, error) {
    resp, err := client.Index("movies").Search(&meilisearch.SearchRequest{
        Query:  q,
        Filter: "rating > 9.0",
        Sort:   []string{"rating:desc"},
        Limit:  10,
    })
    return resp, err
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 meilisearch-js",
              code: `npm install meilisearch`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "写入与搜索",
              code: `import { MeiliSearch } from "meilisearch";

const client = new MeiliSearch({ host: "http://127.0.0.1:${port}" });
const index = client.index("movies");

// 写入文档
await index.addDocuments([
  { id: 1, title: "盗梦空间", year: 2010, genres: ["科幻"], rating: 9.3 },
  { id: 2, title: "星际穿越", year: 2014, genres: ["科幻"], rating: 9.4 },
], { primaryKey: "id" });

// 搜索
const result = await index.search("星际", {
  filter: "rating > 9.0",
  sort: ["rating:desc"],
  limit: 10,
});
console.log(result.hits);

// 删除索引
await client.deleteIndex("movies");`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 meilisearch-python-sdk",
              code: `pip install meilisearch`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "写入与搜索",
              code: `import meilisearch

client = meilisearch.Client("http://127.0.0.1:${port}")
index = client.index("movies")

# 写入文档
index.add_documents([
    {"id": 1, "title": "盗梦空间", "year": 2010, "genres": ["科幻"], "rating": 9.3},
    {"id": 2, "title": "星际穿越", "year": 2014, "genres": ["科幻"], "rating": 9.4},
])

# 搜索
result = index.search("星际", {
    "filter": "rating > 9.0",
    "sort": ["rating:desc"],
    "limit": 10,
})
print(result["hits"])

# 删除索引
client.delete_index("movies")`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "单次导入不要超过 2 MiB",
          value:
            "meilisearch 单次 POST /documents 的请求体上限约为 100MB（取决于具体版本），但建议每批控制在几千到一万条、请求体在几十兆以内。数据量大时用分批导入并把每批压在合理大小，导入速度更快也更稳定。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "Meilisearch 在本地开发时异常顺利，到了生产环境需要留意这些点。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "搜索不到刚写入的文档",
              "写入后立即搜索结果里没有",
              "Meilisearch 异步建索引；用 taskUid 查询任务状态，等 succeeded",
            ],
            [
              "filter 不生效",
              "加了 filter 条件返回空或报错",
              "确认对应字段已被设为 filterableAttributes",
            ],
            [
              "sort 不生效",
              "排序结果不对或报错",
              "确认对应字段已被设为 sortableAttributes",
            ],
            [
              "dev 模式直接暴露",
              "生产环境没设 API Key",
              "切到 production 环境必须先设 master key，确保只有授权请求能访问",
            ],
            [
              "索引占用空间过大",
              "磁盘空间不够",
              "删除不需要的索引；设置合理的 maxTotalIndexSize 限制",
            ],
            [
              "中文搜索不理想",
              "搜中文结果不相关",
              "确认文档内的中文文本是真正的 CJK 字符；检查 searchableAttributes 是否覆盖了中文字段",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "排查命令",
          code: `BASE=http://127.0.0.1:${port}

# 查看版本与状态
curl "$BASE/version"
curl "$BASE/stats"

# 列出所有索引
curl "$BASE/indexes"

# 查看索引详情（文档数、是否正在索引）
curl "$BASE/indexes/movies"

# 查看任务队列（找卡住的任务）
curl "$BASE/tasks?statuses=failed,processing"

# 删除一个索引
curl -X DELETE "$BASE/indexes/movies"`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页展示了索引数量、文档总数和数据库大小；「索引与搜索」标签页集成了创建索引、导入文档、搜索的全部操作，是日常调试最常用的入口；「运行日志」能看到启动报错；做危险操作前建议去「备份恢复」标签页打一个快照。",
        },
      ],
    },
  ];
}
