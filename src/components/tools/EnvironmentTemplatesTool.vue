<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  runtimeOverview,
  runtimeProjectSave,
} from "../../api/services";
import type { RuntimeKind, RuntimeProject, ServiceKind } from "../../types";

type Template = {
  id: string; name: string; category: string; description: string;
  runtime: RuntimeKind; services: ServiceKind[]; env: string[]; color: string;
};
const templates: Template[] = [
  {id:"vue-node-postgres",name:"Vue + Node.js",category:"Web",description:"前后端 Web 项目，使用 PostgreSQL 和 Redis。",runtime:"node",services:["postgres","redis"],env:["DATABASE_URL","REDIS_URL","PORT"],color:"#3d806b"},
  {id:"spring-mysql",name:"Spring Boot",category:"Java",description:"Java API 服务，使用 MySQL、Redis 和 Mailpit。",runtime:"java",services:["mysql","redis","mailpit"],env:["SPRING_DATASOURCE_URL","REDIS_HOST","MAIL_HOST"],color:"#9a533e"},
  {id:"go-nats",name:"Go Microservice",category:"Go",description:"轻量 Go 服务，使用 PostgreSQL 和 NATS。",runtime:"go",services:["postgres","nats"],env:["DATABASE_URL","NATS_URL","HTTP_PORT"],color:"#24768a"},
  {id:"python-mongo",name:"Python API",category:"Python",description:"Python 接口项目，使用 MongoDB 与 Redis。",runtime:"python",services:["mongodb","redis"],env:["MONGODB_URL","REDIS_URL","APP_ENV"],color:"#876b2e"},
  {id:"rust-postgres",name:"Rust Backend",category:"Rust",description:"Rust 后端服务，使用 PostgreSQL 和 NATS。",runtime:"rust",services:["postgres","nats"],env:["DATABASE_URL","NATS_URL","RUST_LOG"],color:"#6e4a37"},
  {id:"flutter-mock",name:"Flutter Client",category:"Mobile",description:"移动端联调环境，搭配 Mock API 和 Mailpit。",runtime:"node",services:["mailpit"],env:["API_BASE_URL","APP_ENV"],color:"#356b96"},
];
const query = ref("");
const category = ref("全部");
const selected = ref<Template | null>(null);
const applying = ref(false);
const notice = ref("");
const error = ref("");
const runtimeVersions = ref<Partial<Record<RuntimeKind,string>>>({});
const categories = computed(()=>["全部",...new Set(templates.map((item)=>item.category))]);
const filtered = computed(()=>templates.filter((item)=>(category.value==="全部"||item.category===category.value)&&`${item.name} ${item.description} ${item.services.join(" ")}`.toLowerCase().includes(query.value.toLowerCase())));
const serviceName: Record<ServiceKind,string> = {redis:"Redis",mysql:"MySQL",postgres:"PostgreSQL",mongodb:"MongoDB",mailpit:"Mailpit",nats:"NATS",kafka:"Kafka Sandbox",meilisearch:"Meilisearch",influxdb:"InfluxDB",minio:"MinIO",rustfs:"RustFS",etcd:"etcd",consul:"Consul",rnacos:"r-nacos",rabbitmq:"RabbitMQ",activemq:"ActiveMQ Classic",nginx:"Nginx",caddy:"Caddy",ftp:"FTP Server"};

async function loadRuntimes() {
  for (const kind of ["go","java","rust","python","node"] as RuntimeKind[]) {
    try { runtimeVersions.value[kind]=(await runtimeOverview(kind)).selectedVersion ?? ""; } catch { runtimeVersions.value[kind]=""; }
  }
}
function emptyProject(path:string,name:string,template:Template):RuntimeProject {
  const version=runtimeVersions.value[template.runtime]||null;
  return {id:"",name,path,description:`基于“${template.name}”环境模板创建`,services:[...template.services],goVersion:template.runtime==="go"?version:null,javaVersion:template.runtime==="java"?version:null,rustVersion:template.runtime==="rust"?version:null,pythonVersion:template.runtime==="python"?version:null,nodeVersion:template.runtime==="node"?version:null,createdAtMillis:0,updatedAtMillis:0};
}
async function useTemplate(template:Template) {
  const path=await open({directory:true,multiple:false,title:`为 ${template.name} 选择项目目录`});
  if(!path)return;
  applying.value=true; error.value="";
  try{
    const name=path.split(/[\\/]/).filter(Boolean).at(-1)||template.name;
    const projects=await runtimeProjectSave(emptyProject(path,name,template));
    const project=projects.find((item)=>item.path===path);
    notice.value=`已创建“${name}”工作区${runtimeVersions.value[template.runtime]?"":"；运行时尚未安装，请稍后在开发环境中选择版本"}`;
    window.dispatchEvent(new CustomEvent("zhiyu:navigate",{detail:{type:"tool",id:"workspace"}}));
    window.setTimeout(() => window.dispatchEvent(new CustomEvent("zhiyu:project-open",{detail:{id:project?.id}})), 0);
  }catch(cause){error.value=String(cause)}finally{applying.value=false}
}
function copyEnv(template:Template){navigator.clipboard.writeText(template.env.map((key)=>`${key}=`).join("\n"));notice.value="环境变量模板已复制"}
function openWorkspace(){window.dispatchEvent(new CustomEvent("zhiyu:navigate",{detail:{type:"tool",id:"workspace"}}))}
onMounted(loadRuntimes);
</script>

<template>
  <header class="detail-header"><div class="detail-identity"><span class="service-logo template-logo">T</span><div><div class="title-line"><h1>环境模板</h1><span>LOCAL RECIPES</span></div><p>从经过整理的本地模板快速创建项目工作区，不下载脚本、不自动执行命令</p></div></div><div class="header-actions"><button @click="openWorkspace">打开项目工作区 ↗</button></div></header>
  <div v-if="notice" class="notice"><span>{{notice}}</span><button @click="notice=''">×</button></div><div v-if="error" class="notice danger"><span>{{error}}</span><button @click="error=''">×</button></div>
  <main class="templates-page">
    <section class="template-toolbar"><input v-model="query" placeholder="搜索技术栈或服务…" /><div><button v-for="item in categories" :key="item" :class="{active:category===item}" @click="category=item">{{item}}</button></div><span>{{filtered.length}} 个模板</span></section>
    <section class="template-grid">
      <article v-for="item in filtered" :key="item.id" :style="{'--template-color':item.color}">
        <div class="template-top"><span>{{item.name.slice(0,1)}}</span><em>{{item.category}}</em></div>
        <h2>{{item.name}}</h2><p>{{item.description}}</p>
        <div class="template-runtime"><small>RUNTIME</small><strong>{{item.runtime==="node"?"Node.js":item.runtime[0].toUpperCase()+item.runtime.slice(1)}}</strong><code>{{runtimeVersions[item.runtime]||"待安装"}}</code></div>
        <div class="template-services"><small>SERVICE STACK</small><span v-for="service in item.services" :key="service">{{serviceName[service]}}</span></div>
        <footer><button @click="copyEnv(item)">复制 .env 示例</button><button class="primary" :disabled="applying" @click="useTemplate(item)">使用模板</button></footer>
      </article>
    </section>
    <section class="template-safety"><strong>模板边界</strong><span>模板只会创建智屿项目配置，记录所需运行时、服务与环境变量名称。</span><span>不会写入业务源码，不会保存密码，也不会自动执行 npm、Maven 或 Shell 脚本。</span></section>
  </main>
</template>

<style scoped>
.template-logo{background:#775582}.templates-page{display:grid;gap:14px;padding:24px 32px 40px}.template-toolbar{display:grid;grid-template-columns:minmax(220px,1fr) auto auto;align-items:center;gap:12px;padding:10px;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.template-toolbar input{height:34px;padding:0 10px}.template-toolbar>div{display:flex}.template-toolbar button{height:30px;padding:0 9px;border:1px solid var(--color-border);background:transparent;color:var(--color-text-muted);font-size:8px}.template-toolbar button.active{border-color:var(--color-accent);background:var(--color-panel-active);color:var(--color-accent)}.template-toolbar>span{color:var(--color-text-muted);font-size:8px}.template-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));border-top:1px solid var(--color-border);border-left:1px solid var(--color-border)}.template-grid>article{position:relative;display:grid;min-height:300px;align-content:start;gap:10px;padding:17px;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);background:var(--color-panel-translucent)}.template-grid>article:before{position:absolute;content:"";inset:0 auto auto 0;width:100%;height:2px;background:var(--template-color)}.template-top{display:flex;align-items:center;justify-content:space-between}.template-top>span{display:grid;width:36px;height:36px;place-items:center;border-radius:50%;background:color-mix(in srgb,var(--template-color) 28%,var(--color-bg-muted));color:var(--template-color);font-weight:700}.template-top em{padding:3px 6px;border:1px solid var(--color-border);color:var(--color-text-muted);font:normal 7px "SFMono-Regular",monospace}.template-grid h2{margin:2px 0 0;font-size:15px}.template-grid p{min-height:32px;margin:0;color:var(--color-text-muted);font-size:8px;line-height:1.6}.template-runtime{display:grid;grid-template-columns:1fr auto;gap:4px;padding:9px;border:1px solid var(--color-border);background:var(--color-bg-muted)}.template-runtime small,.template-services small{grid-column:1/-1;color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.1em}.template-runtime strong{font-size:9px}.template-runtime code{color:var(--color-accent);font-size:8px}.template-services{display:flex;flex-wrap:wrap;gap:5px}.template-services small{width:100%}.template-services span{padding:3px 6px;border:1px solid var(--color-border);color:var(--color-text-secondary);font-size:7px}.template-grid footer{display:flex;justify-content:flex-end;gap:7px;margin-top:auto;padding-top:5px}.template-grid footer button{height:29px;padding:0 9px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:8px}.template-grid footer button.primary{border-color:var(--color-control-primary);background:var(--color-control-primary);color:#fff}.template-safety{display:flex;align-items:center;gap:14px;padding:12px 14px;border:1px solid var(--color-border);background:var(--color-bg-muted);font-size:8px}.template-safety span{color:var(--color-text-muted)}@media(max-width:1050px){.template-grid{grid-template-columns:repeat(2,1fr)}.template-toolbar{grid-template-columns:1fr}.template-safety{align-items:flex-start;flex-direction:column;gap:5px}}
</style>
