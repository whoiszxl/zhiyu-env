import type { DocChapter } from "../docTypes";

export function buildMeilisearchDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet Meilisearch",
      navHint: "Index · Full-text search",
      title: "What is Meilisearch",
      intro:
        "Meilisearch is a full-text search engine built for application developers. Write JSON documents into an index and get prefix search and typo-tolerant results through an HTTP API.",
      blocks: [
        {
          kind: "text",
          value:
            "Meilisearch solves problems similar to Elasticsearch but with a completely different design philosophy. Elasticsearch is extremely feature-rich but costly to deploy and tune, while Meilisearch aims for \"out of the box\"—a single binary, zero configuration, millisecond-level search, and built-in Chinese tokenization. It's enough for the vast majority of internal search, docs search, and e-commerce product search, without having to wrestle with Elasticsearch's JVM tuning.",
        },
        {
          kind: "text",
          value: "Its core capabilities:",
        },
        {
          kind: "list",
          items: [
            "Prefix search and typo tolerance: typing \"zhnag\" still finds documents related to \"张三\".",
            "Filters and sorting: filter and sort precisely by fields like price, time, and category.",
            "Faceted aggregation: search results include category counts, useful for building \"filter by brand/price range\" UI.",
            "Asynchronous indexing: indexing happens in the background after writes, and write endpoints return in seconds.",
            "Multilingual support: built-in CJK tokenization for Chinese, Japanese, Korean, etc., no extra plugins required.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Local config", "Notes"],
          rows: [
            ["HTTP address", `http://127.0.0.1:${port}`, "Address for your app and SDKs to connect"],
            ["Environment", "development", "Local development mode"],
            ["API Key", "Not set (dev mode)", "Must be set when switching to production"],
            ["Analytics", "Disabled", "No anonymous telemetry is sent"],
            ["Data directory", "~/.devbox/instances/meilisearch/default/data", "Index files"],
          ],
        },
        {
          kind: "table",
          head: ["", "Meilisearch", "Elasticsearch", "Algolia"],
          rows: [
            ["Deployment", "Single binary", "Java cluster + JVM tuning", "SaaS, no deployment"],
            ["Startup speed", "Seconds", "Minutes", "No startup needed"],
            ["Chinese tokenization", "Built-in", "Requires plugins like IK", "Built-in"],
            ["Query latency", "Milliseconds", "Milliseconds (depends on index size)", "Milliseconds"],
            ["Configuration complexity", "Very low, works with zero config", "High, needs extensive tuning", "Medium"],
            ["Best for", "In-app search at small to medium scale", "Massive logs, monitoring, full-text retrieval", "External search products"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "An index is like a table designed for search",
          value:
            "Each index holds a set of JSON documents and uses one field as the primary key. All document fields are searchable by default, and you can later use the API to adjust which fields are searchable, filterable, and sortable.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quickstart",
      navHint: "Build an index · Run a search",
      title: "Create an index and search",
      intro:
        "Zhiyu has already installed and started Meilisearch. The \"Indexes & Search\" tab integrates index management and search debugging—no need to switch to external tools.",
      blocks: [
        {
          kind: "list",
          items: [
            "Check the \"Overview\" tab to confirm the status is \"Running\".",
            "The \"Indexes & Search\" tab lets you create/select indexes, import JSON documents, and run searches directly—no code required.",
            "When you start coding, the curl commands below can be run directly in a terminal or from your code.",
          ],
        },
        {
          kind: "table",
          head: ["Parameter", "Value", "Notes"],
          rows: [
            ["Host", "127.0.0.1", "Listens on localhost only"],
            ["Port", String(port), "HTTP API port"],
            ["API Key", "(empty, dev mode)", "No authentication required for local development"],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Create an index and write documents",
          code: `BASE=http://127.0.0.1:${port}

# Create an index (set the primary key field)
curl -X POST "$BASE/indexes" \\
  -H "Content-Type: application/json" \\
  -d '{"uid":"movies","primaryKey":"id"}'

# Write a batch of documents
curl -X POST "$BASE/indexes/movies/documents" \\
  -H "Content-Type: application/json" \\
  -d '[
  {"id":1,"title":"盗梦空间","year":2010,"genres":["科幻","动作"],"rating":9.3},
  {"id":2,"title":"星际穿越","year":2014,"genres":["科幻","剧情"],"rating":9.4},
  {"id":3,"title":"泰坦尼克号","year":1997,"genres":["爱情","剧情"],"rating":9.5},
  {"id":4,"title":"千与千寻","year":2001,"genres":["动画","奇幻"],"rating":9.4},
  {"id":5,"title":"肖申克的救赎","year":1994,"genres":["剧情"],"rating":9.7}
]'

# Check the write task status (get taskUid from the previous response)
curl "$BASE/indexes/movies/tasks"

# Once the task status becomes succeeded, you can search`,
        },
        {
          kind: "text",
          value: "Wait a second or two after writing for indexing to finish, then run a search:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Search",
          code: `# Basic search
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"星际"}'

# In the search response:
#   hits          the list of matched documents
#   estimatedTotalHits  total number of matches
#   processingTimeMs    query time in milliseconds

# Search with filter and sort
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
          title: "The search endpoint is POST, not GET",
          value:
            "Meilisearch's search endpoint expects a POST with JSON parameters, which differs from most REST API conventions. GET /indexes/movies/search?q=xxx works too, but a JSON body is cleaner once you have many parameters.",
        },
      ],
    },

    {
      id: "features",
      navLabel: "Search features",
      navHint: "Filter · Sort · Typo tolerance",
      title: "Get the most out of search",
      intro:
        "Meilisearch already produces decent search results with zero configuration, but tuning a few key settings raises the experience to another level.",
      blocks: [
        {
          kind: "text",
          value:
            "By default, Meilisearch treats every document field as \"searchable\". In real projects, some fields don't need to be searched (like id or createdAt). Narrowing the searchable attributes noticeably improves search precision:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Restrict searchable fields",
          code: `curl -X PUT "$BASE/indexes/movies/settings/searchable-attributes" \\
  -H "Content-Type: application/json" \\
  -d '["title","genres"]'

# Inspect the current setting
curl "$BASE/indexes/movies/settings/searchable-attributes"`,
        },
        {
          kind: "text",
          value:
            "Similarly, filtering and sorting require you to explicitly declare which fields are allowed. Fields not declared cannot be used in filter or sort:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Set filterable and sortable fields",
          code: `curl -X PUT "$BASE/indexes/movies/settings/filterable-attributes" \\
  -H "Content-Type: application/json" \\
  -d '["year","rating","genres"]'

curl -X PUT "$BASE/indexes/movies/settings/sortable-attributes" \\
  -H "Content-Type: application/json" \\
  -d '["year","rating"]'

# Only then can you use:
#   filter: "year >= 2000 AND rating > 9.0"
#   sort: ["year:desc", "rating:asc"]

# Boolean filters are also supported
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"","filter":"genres IN [科幻, 动画]"}'`,
        },
        {
          kind: "text",
          value: "Sorting and faceted aggregation are often used together:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Sort and search examples",
          code: `# Sort by rating descending
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"电影","sort":["rating:desc"],"limit":10}'

# facets returns category counts (great for sidebar filters)
curl -X POST "$BASE/indexes/movies/search" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"","facets":["genres","year"]}'
# The response's facetDistribution contains the distribution for genres and year`,
        },
        {
          kind: "table",
          head: ["Setting", "Purpose", "Recommendation"],
          rows: [
            ["searchableAttributes", "Restrict which fields participate in full-text search", "Only include fields users actually search; exclude id and internal fields"],
            ["filterableAttributes", "Which fields can be used in filter conditions", "Include fields like year / rating / status that need filtering"],
            ["sortableAttributes", "Which fields can be used for sort", "Include numeric and date fields"],
            ["rankingRules", "Tune relevance ranking weights", "The defaults are already great, avoid changing them lightly"],
            ["typoTolerance", "Toggle and parameters for typo tolerance", "On by default; typically no need to change for Chinese content"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Every settings change triggers a full reindex",
          value:
            "After modifying settings like searchableAttributes or filterableAttributes, Meilisearch starts an async task to reindex every document. This takes a while for large datasets, but the write endpoint remains available during the reindex.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language clients",
      navHint: "Java · Go · TS · Python",
      title: "Connect from your project",
      intro: `The following configurations all point to the local http://127.0.0.1:${port}. Every language SDK wraps the REST API, so if a language has no official SDK you can still call the HTTP API directly.`,
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
              caption: "Write and search",
              code: `import com.meilisearch.sdk.*;
import com.google.gson.JsonObject;

Config config = new Config("http://127.0.0.1:${port}", null);
Client client = new Client(config);

// Write a document
JsonObject doc = new JsonObject();
doc.addProperty("id", 1);
doc.addProperty("title", "盗梦空间");
doc.addProperty("year", 2010);
client.index("movies").addDocuments("[{...}]");

// Search
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
              caption: "Install meilisearch-go",
              code: `go get github.com/meilisearch/meilisearch-go`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Write and search",
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
              caption: "Install meilisearch-js",
              code: `npm install meilisearch`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Write and search",
              code: `import { MeiliSearch } from "meilisearch";

const client = new MeiliSearch({ host: "http://127.0.0.1:${port}" });
const index = client.index("movies");

// Write documents
await index.addDocuments([
  { id: 1, title: "盗梦空间", year: 2010, genres: ["科幻"], rating: 9.3 },
  { id: 2, title: "星际穿越", year: 2014, genres: ["科幻"], rating: 9.4 },
], { primaryKey: "id" });

// Search
const result = await index.search("星际", {
  filter: "rating > 9.0",
  sort: ["rating:desc"],
  limit: 10,
});
console.log(result.hits);

// Delete the index
await client.deleteIndex("movies");`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install meilisearch-python-sdk",
              code: `pip install meilisearch`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Write and search",
              code: `import meilisearch

client = meilisearch.Client("http://127.0.0.1:${port}")
index = client.index("movies")

# Write documents
index.add_documents([
    {"id": 1, "title": "盗梦空间", "year": 2010, "genres": ["科幻"], "rating": 9.3},
    {"id": 2, "title": "星际穿越", "year": 2014, "genres": ["科幻"], "rating": 9.4},
])

# Search
result = index.search("星际", {
    "filter": "rating > 9.0",
    "sort": ["rating:desc"],
    "limit": 10,
})
print(result["hits"])

# Delete the index
client.delete_index("movies")`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Keep a single import under 2 MiB",
          value:
            "Meilisearch's POST /documents request body limit is around 100MB (depending on version), but it's recommended to keep each batch to a few thousand up to ten thousand documents with a request body of tens of MB at most. For large datasets, batch imports with reasonable sizes are both faster and more stable.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls & tuning",
      navHint: "Troubleshooting · Common issues",
      title: "Things to know before going live",
      intro: "Meilisearch is remarkably smooth in local development, but there are a few things to watch out for in production.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Fix"],
          rows: [
            [
              "Just-written documents aren't searchable",
              "Search right after writing returns no results",
              "Meilisearch indexes asynchronously; query the task with its taskUid and wait for succeeded",
            ],
            [
              "filter has no effect",
              "Adding a filter condition returns empty results or errors",
              "Confirm the field is registered in filterableAttributes",
            ],
            [
              "sort has no effect",
              "Sort results are wrong or the request errors",
              "Confirm the field is registered in sortableAttributes",
            ],
            [
              "dev mode is exposed",
              "No API Key set in production",
              "Set a master key before switching to production so only authorized requests can access it",
            ],
            [
              "Indexes take too much space",
              "Running out of disk",
              "Delete unused indexes; set a reasonable maxTotalIndexSize limit",
            ],
            [
              "Chinese search is unsatisfying",
              "Chinese queries return irrelevant results",
              "Make sure the Chinese text in documents is real CJK characters; check that searchableAttributes covers the Chinese fields",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Troubleshooting commands",
          code: `BASE=http://127.0.0.1:${port}

# View version and stats
curl "$BASE/version"
curl "$BASE/stats"

# List all indexes
curl "$BASE/indexes"

# Inspect an index (document count, whether indexing is in progress)
curl "$BASE/indexes/movies"

# Inspect the task queue (find stuck tasks)
curl "$BASE/tasks?statuses=failed,processing"

# Delete an index
curl -X DELETE "$BASE/indexes/movies"`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do this in Zhiyu",
          value:
            "The \"Overview\" tab shows the number of indexes, total document count, and database size; the \"Indexes & Search\" tab integrates creating indexes, importing documents, and searching—the everyday entry point for debugging; \"Runtime logs\" surface startup errors; before any risky operation, head to the \"Backup & restore\" tab and take a snapshot.",
        },
      ],
    },
  ];
}
