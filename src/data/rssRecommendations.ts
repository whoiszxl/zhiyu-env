export type RssRecommendationCategory =
  | "chinese"
  | "programming"
  | "ai"
  | "engineering";

export interface RssRecommendation {
  name: string;
  url: string;
  category: RssRecommendationCategory;
  source: "official" | "community";
}

// This is a local discovery catalog only. Nothing is subscribed until the user
// explicitly selects a feed and confirms it in the add-feed dialog.
export const RSS_RECOMMENDATIONS: readonly RssRecommendation[] = [
  { name: "知乎日报", url: "https://rsshub.bestblogs.dev/zhihu/daily", category: "chinese", source: "community" },
  { name: "阮一峰的网络日志", url: "https://www.ruanyifeng.com/blog/atom.xml", category: "chinese", source: "official" },
  { name: "少数派", url: "https://sspai.com/feed", category: "chinese", source: "official" },
  { name: "少数派 Matrix", url: "https://plink.anyfeeder.com/ssapi/matrix", category: "chinese", source: "community" },
  { name: "美团技术团队", url: "https://tech.meituan.com/rss.xml", category: "chinese", source: "official" },
  { name: "V2EX", url: "https://www.v2ex.com/index.xml", category: "chinese", source: "official" },
  { name: "V2EX · 技术", url: "https://www.v2ex.com/feed/tab/tech.xml", category: "chinese", source: "official" },
  { name: "酷壳 CoolShell", url: "https://coolshell.cn/feed", category: "chinese", source: "official" },
  { name: "爱范儿", url: "https://www.ifanr.com/feed", category: "chinese", source: "official" },
  { name: "小众软件", url: "https://www.appinn.com/feed/", category: "chinese", source: "official" },
  { name: "虎嗅", url: "https://rss.huxiu.com/", category: "chinese", source: "official" },
  { name: "36氪", url: "https://36kr.com/feed", category: "chinese", source: "official" },
  { name: "罗辑思维", url: "https://plink.anyfeeder.com/weixin/luojisw", category: "chinese", source: "community" },
  { name: "人人都是产品经理", url: "https://plink.anyfeeder.com/weixin/woshipm", category: "chinese", source: "community" },
  { name: "维基百科优良条目", url: "https://zh.wikipedia.org/w/api.php?action=featuredfeed&feed=good&feedformat=atom", category: "chinese", source: "official" },
  { name: "人民网 · 国际新闻", url: "https://plink.anyfeeder.com/people/world", category: "chinese", source: "community" },
  { name: "经济学人", url: "https://plink.anyfeeder.com/weixin/theeconomist", category: "chinese", source: "community" },
  { name: "果壳网 · 科学人", url: "https://plink.anyfeeder.com/guokr/scientific", category: "chinese", source: "community" },

  { name: "Inside Java", url: "https://inside.java/feed.xml", category: "programming", source: "official" },
  { name: "Spring Blog", url: "https://spring.io/blog.atom", category: "programming", source: "official" },
  { name: "The Go Blog", url: "https://go.dev/blog/feed.atom", category: "programming", source: "official" },
  { name: "Rust Blog", url: "https://blog.rust-lang.org/feed.xml", category: "programming", source: "official" },
  { name: "Node.js Blog", url: "https://nodejs.org/en/feed/blog.xml", category: "programming", source: "official" },
  { name: "Python Insider", url: "https://blog.python.org/rss.xml", category: "programming", source: "official" },
  { name: "Kotlin Blog", url: "https://blog.jetbrains.com/kotlin/feed/", category: "programming", source: "official" },

  { name: "OpenAI News", url: "https://openai.com/news/rss.xml", category: "ai", source: "official" },
  { name: "Anthropic News", url: "https://rsshub.bestblogs.dev/anthropic/news", category: "ai", source: "community" },
  { name: "Google DeepMind", url: "https://deepmind.google/blog/rss.xml", category: "ai", source: "official" },
  { name: "Hugging Face Blog", url: "https://huggingface.co/blog/feed.xml", category: "ai", source: "official" },
  { name: "arXiv · Artificial Intelligence", url: "https://export.arxiv.org/rss/cs.AI", category: "ai", source: "official" },

  { name: "GitHub Blog", url: "https://github.blog/feed/", category: "engineering", source: "official" },
  { name: "Cloudflare Blog", url: "https://blog.cloudflare.com/rss/", category: "engineering", source: "official" },
  { name: "Kubernetes Blog", url: "https://kubernetes.io/feed.xml", category: "engineering", source: "official" },
  { name: "Stack Overflow Blog", url: "https://stackoverflow.blog/feed/", category: "engineering", source: "official" },
  { name: "Meta Engineering", url: "https://engineering.fb.com/feed/", category: "engineering", source: "official" },
  { name: "Microsoft Developer Blogs", url: "https://devblogs.microsoft.com/feed/", category: "engineering", source: "official" },
  { name: "Mozilla Hacks", url: "https://hacks.mozilla.org/feed/", category: "engineering", source: "official" },
];
