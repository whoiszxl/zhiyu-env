<script setup lang="ts">
import { computed, ref } from "vue";
import { decodeJwt, inspectJwk, signJwtHmac, verifyJwtHmac } from "../../api/tools";
import type {
  HmacAlgorithm,
  JwkInspection,
  JwtDecoded,
  JwtVerifyResult,
  SecretEncoding,
  TokenStatus,
} from "../../types";

type Mode = "decode" | "sign" | "jwk";

const MODES: Array<[Mode, string, string]> = [
  ["decode", "解码与验签", "查看内容 · 校验签名"],
  ["sign", "生成测试 Token", "HS256 / 384 / 512"],
  ["jwk", "JWK / JWKS", "密钥集合查看"],
];

const ALGORITHMS: HmacAlgorithm[] = ["HS256", "HS384", "HS512"];

const STATUS_LABEL: Record<TokenStatus, string> = {
  active: "有效",
  expired: "已过期",
  notYetValid: "尚未生效",
  noTimeLimit: "无过期时间",
};

const DEFAULT_PAYLOAD = `{
  "sub": "1001",
  "name": "张三",
  "role": "admin",
  "iat": 1700000000,
  "exp": 1900000000
}`;

const mode = ref<Mode>("decode");
const error = ref("");
const notice = ref("");

// ── 解码与验签 ──────────────────────────────────────────────
const token = ref("");
const decoded = ref<JwtDecoded | null>(null);
const decoding = ref(false);

const secret = ref("");
const secretEncoding = ref<SecretEncoding>("utf8");
const verifyResult = ref<JwtVerifyResult | null>(null);
const verifying = ref(false);

// ── 生成测试 Token ──────────────────────────────────────────
const signPayload = ref(DEFAULT_PAYLOAD);
const signAlgorithm = ref<HmacAlgorithm>("HS256");
const signSecret = ref("zhiyu-dev-secret");
const signEncoding = ref<SecretEncoding>("utf8");
const signKeyId = ref("");
const signedToken = ref("");
const signing = ref(false);

// ── JWK ────────────────────────────────────────────────────
const jwkInput = ref("");
const jwkResult = ref<JwkInspection | null>(null);
const inspecting = ref(false);

const authorizationHeader = computed(() =>
  token.value.trim() ? `Authorization: Bearer ${cleanToken(token.value)}` : "",
);

/** 去掉粘贴时可能带上的请求头前缀，只保留 Token 本身。 */
function cleanToken(raw: string): string {
  return raw
    .trim()
    .replace(/^Authorization:\s*/i, "")
    .replace(/^Bearer\s+/i, "")
    .trim();
}

function formatTime(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString("zh-CN", { hour12: false });
}

function formatOffset(offsetSeconds: number): string {
  const absolute = Math.abs(offsetSeconds);
  const unit =
    absolute < 60
      ? `${absolute} 秒`
      : absolute < 3600
        ? `${Math.floor(absolute / 60)} 分钟`
        : absolute < 86400
          ? `${Math.floor(absolute / 3600)} 小时`
          : `${Math.floor(absolute / 86400)} 天`;
  return offsetSeconds >= 0 ? `${unit}后` : `${unit}前`;
}

async function runDecode() {
  if (decoding.value) return;
  decoding.value = true;
  error.value = "";
  verifyResult.value = null;
  try {
    decoded.value = await decodeJwt(token.value);
  } catch (cause) {
    decoded.value = null;
    error.value = String(cause);
  } finally {
    decoding.value = false;
  }
}

async function runVerify() {
  if (verifying.value) return;
  verifying.value = true;
  error.value = "";
  try {
    verifyResult.value = await verifyJwtHmac(
      token.value,
      secret.value,
      secretEncoding.value,
    );
  } catch (cause) {
    verifyResult.value = null;
    error.value = String(cause);
  } finally {
    verifying.value = false;
  }
}

async function runSign() {
  if (signing.value) return;
  signing.value = true;
  error.value = "";
  notice.value = "";
  try {
    signedToken.value = await signJwtHmac(
      signPayload.value,
      signAlgorithm.value,
      signSecret.value,
      signEncoding.value,
      signKeyId.value.trim() || null,
    );
  } catch (cause) {
    signedToken.value = "";
    error.value = String(cause);
  } finally {
    signing.value = false;
  }
}

/** 把刚生成的 Token 送去解码页，方便立刻检查内容与验签。 */
function inspectGeneratedToken() {
  token.value = signedToken.value;
  secret.value = signSecret.value;
  secretEncoding.value = signEncoding.value;
  mode.value = "decode";
  void runDecode();
}

async function runInspectJwk() {
  if (inspecting.value) return;
  inspecting.value = true;
  error.value = "";
  try {
    jwkResult.value = await inspectJwk(jwkInput.value);
  } catch (cause) {
    jwkResult.value = null;
    error.value = String(cause);
  } finally {
    inspecting.value = false;
  }
}

async function copy(text: string, label: string) {
  try {
    await navigator.clipboard.writeText(text);
    notice.value = `${label}已复制到剪贴板`;
  } catch {
    error.value = "复制失败，请手动选中内容复制";
  }
}

function switchMode(next: Mode) {
  mode.value = next;
  error.value = "";
  notice.value = "";
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo jwt">J</span>
      <div>
        <div class="title-line">
          <h1>JWT 调试器</h1>
          <span>OFFLINE ONLY</span>
        </div>
        <p>解码、验签与生成测试 Token，全部在本机完成，密钥和 Token 不会发往任何服务</p>
      </div>
    </div>
  </header>

  <div v-if="notice || error" class="notice" :class="{ danger: error }">
    <span>{{ error || notice }}</span>
    <button type="button" @click="notice = error = ''">×</button>
  </div>

  <section class="tool-page">
    <nav class="tool-modes">
      <button
        v-for="item in MODES"
        :key="item[0]"
        type="button"
        :class="{ active: mode === item[0] }"
        @click="switchMode(item[0])"
      >
        <strong>{{ item[1] }}</strong>
        <small>{{ item[2] }}</small>
      </button>
    </nav>

    <!-- 解码与验签 -->
    <div v-if="mode === 'decode'" class="tool-panel">
      <div class="tool-pane">
        <div class="tool-pane-head">
          <p>TOKEN</p>
          <button
            v-if="authorizationHeader"
            type="button"
            class="tool-copy"
            @click="copy(authorizationHeader, 'Authorization 请求头')"
          >
            复制 Authorization
          </button>
        </div>
        <textarea
          v-model="token"
          class="jwt-input"
          spellcheck="false"
          placeholder="粘贴 JWT，可直接带 Bearer 前缀或整行 Authorization 请求头"
        ></textarea>
      </div>

      <div class="tool-controls">
        <button class="primary" type="button" :disabled="decoding" @click="runDecode">
          <span v-if="decoding" class="spinner"></span>
          {{ decoding ? "解析中" : "解码" }}
        </button>
        <label class="grow">
          验签密钥
          <input
            v-model="secret"
            type="text"
            spellcheck="false"
            placeholder="HMAC 密钥，仅用于本机校验"
          />
        </label>
        <label>
          密钥编码
          <select v-model="secretEncoding">
            <option value="utf8">文本</option>
            <option value="base64">Base64</option>
          </select>
        </label>
        <button type="button" :disabled="verifying || !token" @click="runVerify">
          <span v-if="verifying" class="spinner"></span>
          {{ verifying ? "校验中" : "验证签名" }}
        </button>
      </div>

      <div
        v-if="verifyResult"
        class="jwt-verdict"
        :class="verifyResult.valid ? 'ok' : 'bad'"
      >
        <strong>{{ verifyResult.valid ? "✓ 签名有效" : "✕ 签名无效" }}</strong>
        <span>{{ verifyResult.detail }}</span>
      </div>

      <template v-if="decoded">
        <div class="jwt-status-row">
          <span class="jwt-status" :class="decoded.status">
            {{ STATUS_LABEL[decoded.status] }}
          </span>
          <span class="jwt-status-detail">{{ decoded.statusDetail }}</span>
          <span class="jwt-chip">{{ decoded.algorithm }}</span>
          <span v-if="decoded.tokenType" class="jwt-chip">{{ decoded.tokenType }}</span>
          <span v-if="decoded.keyId" class="jwt-chip">kid: {{ decoded.keyId }}</span>
        </div>

        <div v-for="(warning, index) in decoded.warnings" :key="index" class="tool-warning">
          {{ warning }}
        </div>

        <div class="tool-split">
          <div class="tool-pane">
            <div class="tool-pane-head">
              <p>HEADER</p>
              <button type="button" class="tool-copy" @click="copy(decoded.header, '头部')">
                复制
              </button>
            </div>
            <pre class="tool-output">{{ decoded.header }}</pre>
          </div>
          <div class="tool-pane">
            <div class="tool-pane-head">
              <p>PAYLOAD</p>
              <button type="button" class="tool-copy" @click="copy(decoded.payload, '载荷')">
                复制
              </button>
            </div>
            <pre class="tool-output">{{ decoded.payload }}</pre>
          </div>
        </div>

        <div v-if="decoded.timeClaims.length" class="tool-table-wrap">
          <table class="tool-table">
            <thead>
              <tr>
                <th>声明</th>
                <th>含义</th>
                <th>时间</th>
                <th>距现在</th>
                <th>说明</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="claim in decoded.timeClaims" :key="claim.name">
                <td><code>{{ claim.name }}</code></td>
                <td>{{ claim.label }}</td>
                <td>{{ formatTime(claim.value) }}</td>
                <td>{{ formatOffset(claim.offsetSeconds) }}</td>
                <td>{{ claim.description }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="decoded.registeredClaims.length" class="tool-table-wrap">
          <table class="tool-table">
            <thead>
              <tr>
                <th>声明</th>
                <th>含义</th>
                <th>值</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="claim in decoded.registeredClaims" :key="claim.name">
                <td><code>{{ claim.name }}</code></td>
                <td>{{ claim.label }}</td>
                <td>{{ claim.value }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>
    </div>

    <!-- 生成测试 Token -->
    <div v-else-if="mode === 'sign'" class="tool-panel">
      <div class="tool-controls">
        <label>
          算法
          <select v-model="signAlgorithm">
            <option v-for="item in ALGORITHMS" :key="item" :value="item">{{ item }}</option>
          </select>
        </label>
        <label class="grow">
          密钥
          <input v-model="signSecret" type="text" spellcheck="false" />
        </label>
        <label>
          密钥编码
          <select v-model="signEncoding">
            <option value="utf8">文本</option>
            <option value="base64">Base64</option>
          </select>
        </label>
        <label>
          kid（可选）
          <input v-model="signKeyId" type="text" spellcheck="false" placeholder="留空则不写入" />
        </label>
        <button class="primary" type="button" :disabled="signing" @click="runSign">
          <span v-if="signing" class="spinner"></span>
          {{ signing ? "签发中" : "生成 Token" }}
        </button>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head"><p>PAYLOAD</p></div>
          <textarea v-model="signPayload" spellcheck="false"></textarea>
        </div>
        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>TOKEN</p>
            <button
              v-if="signedToken"
              type="button"
              class="tool-copy"
              @click="copy(signedToken, 'Token')"
            >
              复制
            </button>
          </div>
          <pre v-if="signedToken" class="tool-output jwt-token-output">{{ signedToken }}</pre>
          <div v-else class="tool-empty">填好载荷与密钥后点击「生成 Token」</div>
          <div v-if="signedToken" class="jwt-actions">
            <button type="button" @click="inspectGeneratedToken">送去解码页检查</button>
            <button
              type="button"
              @click="copy(`Authorization: Bearer ${signedToken}`, 'Authorization 请求头')"
            >
              复制 Authorization
            </button>
          </div>
        </div>
      </div>

      <p class="tool-note">
        生成的 Token 仅供本地联调使用。载荷里的 <code>exp</code> 是 Unix 秒，
        不写 <code>exp</code> 就是永不过期的 Token，请不要用于任何真实环境。
      </p>
    </div>

    <!-- JWK / JWKS -->
    <div v-else class="tool-panel">
      <div class="tool-controls">
        <button class="primary" type="button" :disabled="inspecting" @click="runInspectJwk">
          <span v-if="inspecting" class="spinner"></span>
          {{ inspecting ? "解析中" : "解析密钥" }}
        </button>
        <span v-if="jwkResult" class="tool-summary">
          {{ jwkResult.source === "jwks" ? "密钥集合" : "单个密钥" }} · 共
          {{ jwkResult.count }} 个
        </span>
      </div>

      <div class="tool-pane">
        <div class="tool-pane-head"><p>JWK / JWKS JSON</p></div>
        <textarea
          v-model="jwkInput"
          spellcheck="false"
          placeholder='粘贴单个 JWK 或形如 {"keys":[...]} 的密钥集合'
        ></textarea>
      </div>

      <div v-for="(warning, index) in jwkResult?.warnings ?? []" :key="index" class="tool-warning">
        {{ warning }}
      </div>

      <div v-if="jwkResult" class="tool-table-wrap">
        <table class="tool-table">
          <thead>
            <tr>
              <th>kid</th>
              <th>类型</th>
              <th>算法</th>
              <th>用途</th>
              <th>摘要</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(key, index) in jwkResult.keys" :key="key.keyId ?? index">
              <td>
                <code v-if="key.keyId">{{ key.keyId }}</code>
                <span v-else class="jwt-muted">未设置</span>
              </td>
              <td>{{ key.keyType }}</td>
              <td>{{ key.algorithm ?? "—" }}</td>
              <td>{{ key.usage ?? "—" }}</td>
              <td>
                {{ key.summary }}
                <span v-if="key.containsPrivateMaterial" class="diff-badge removed">含私钥</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <p class="tool-note">
        JWKS 是服务端公开的公钥集合，验签方按 Token 头部的 <code>kid</code>
        找到对应公钥。这里只做本地解析，不会去请求任何 JWKS 地址。
      </p>
    </div>
  </section>
</template>
