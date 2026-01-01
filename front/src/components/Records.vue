<script setup lang="ts">
import { useStorage } from '@vueuse/core';
import {
  ArrowLeft,
  Copy,
  Link as LinkIcon,
  Lock,
  Pencil,
  Play,
  RefreshCw,
  Share2,
  Trash2,
  X,
} from 'lucide-vue-next';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import {
  deleteGameTemplate,
  listRecords,
  type RecordsListItem,
  shareGame,
} from '../api';
import { WavyBackground } from './ui/wavy-background';

const router = useRouter();

/** GLM 的默认请求地址（用于判定“是否被修改”） */
const DEFAULT_GLM_BASE_URL =
  'https://open.bigmodel.cn/api/paas/v4/chat/completions';
/** GLM 的默认模型（用于判定“是否被修改”） */
const DEFAULT_GLM_MODEL = 'glm-4.6v-flash';

const glmBaseUrl = useStorage('mg_glm_base_url', DEFAULT_GLM_BASE_URL);
const glmModel = useStorage('mg_glm_model', DEFAULT_GLM_MODEL);

/**
 * 数据安全锁：当用户自行修改模型配置时，禁用分享与设计功能。
 */
const securityLocked = computed(() => {
  const baseUrlTouched = glmBaseUrl.value.trim() !== DEFAULT_GLM_BASE_URL;
  const modelTouched = glmModel.value.trim() !== DEFAULT_GLM_MODEL;
  return baseUrlTouched || modelTouched;
});

const recordIds = useStorage<string[]>('mg_record_ids', []);

const items = ref<RecordsListItem[]>([]);
const isLoading = ref(false);
const error = ref('');
const busyItemId = ref<string | null>(null);
const toast = ref<{ text: string; kind: 'info' | 'success' | 'error' } | null>(
  null,
);
let toastTimer: ReturnType<typeof setTimeout> | null = null;

onUnmounted(() => {
  if (toastTimer) clearTimeout(toastTimer);
});

type ConfirmModalState = {
  title: string;
  message: string;
  confirmText: string;
  cancelText: string;
  kind: 'danger' | 'info';
  onConfirm: () => void;
};

const confirmModal = ref<ConfirmModalState | null>(null);

const openConfirm = (state: ConfirmModalState) => {
  confirmModal.value = state;
};

const closeConfirm = () => {
  confirmModal.value = null;
};

const runConfirm = () => {
  const action = confirmModal.value?.onConfirm;
  confirmModal.value = null;
  action?.();
};

const uniqueIds = computed(() => {
  const seen = new Set<string>();
  const ids: string[] = [];
  for (const raw of recordIds.value) {
    const id = String(raw || '').trim();
    if (!id) continue;
    if (seen.has(id)) continue;
    seen.add(id);
    ids.push(id);
  }
  return ids;
});

/**
 * 统一展示提示信息。
 */
const showToast = (text: string, kind: 'info' | 'success' | 'error') => {
  toast.value = { text, kind };
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = null;
  }, 2200);
};

/**
 * 格式化分享时间字符串。
 */
const formatSharedAt = (value: string) => {
  const t = new Date(value);
  if (Number.isNaN(t.getTime())) return value;
  return t.toLocaleString();
};

/**
 * 从 meta.genre 推导展示用 tags。
 */
const deriveTags = (item: RecordsListItem) => {
  const raw = String(item.genre || '').trim();
  const parts = raw
    .split(/[/|,]/g)
    .map((s) => s.trim())
    .filter(Boolean);

  const tags = parts.slice(0, 4);

  const lang = String(item.language || '').trim();
  if (lang) tags.push(lang);

  return tags;
};

/**
 * 刷新历史记录列表（仅拉取列表展示字段）。
 */
const refresh = async () => {
  if (isLoading.value) return;
  error.value = '';
  isLoading.value = true;
  try {
    const data = await listRecords(uniqueIds.value);
    items.value = data;
  } catch (e: unknown) {
    console.error(e);
    error.value = e instanceof Error ? e.message : '加载失败';
  } finally {
    isLoading.value = false;
  }
};

/**
 * 返回首页。
 */
const goHome = () => {
  router.push('/');
};

/**
 * 从前端历史记录中移除（不会影响后端数据）。
 */
const removeLocal = (id: string) => {
  recordIds.value = recordIds.value.filter((x) => x !== id);
  items.value = items.value.filter((x) => x.id !== id);
  showToast('已从本地历史记录移除', 'success');
};

/**
 * 请求“仅移除列表”（本地移除）操作，并弹出确认框。
 */
const requestRemoveLocal = (item: RecordsListItem) => {
  openConfirm({
    title: '仅移除列表',
    message:
      '该操作只会把这条记录从你的浏览器历史列表中移除，不会删除服务端数据。\n\n确定继续吗？',
    confirmText: '移除',
    cancelText: '取消',
    kind: 'info',
    onConfirm: () => {
      removeLocal(item.id);
    },
  });
};

const performDelete = async (item: RecordsListItem) => {
  if (busyItemId.value) return;
  busyItemId.value = item.id;
  try {
    await deleteGameTemplate(item.requestId);
    recordIds.value = recordIds.value.filter((x) => x !== item.id);
    items.value = items.value.filter((x) => x.id !== item.id);
    showToast('已删除该剧情（服务端）', 'success');
  } catch (e: unknown) {
    console.error(e);
    showToast(e instanceof Error ? e.message : '删除失败', 'error');
  } finally {
    busyItemId.value = null;
  }
};

const deleteRemote = (item: RecordsListItem) => {
  openConfirm({
    title: '删除剧情',
    message:
      '该操作会删除服务端保存的剧情数据，并同步删除分享记录与游玩记录。\n\n此操作不可恢复，确定继续吗？',
    confirmText: '删除',
    cancelText: '取消',
    kind: 'danger',
    onConfirm: () => {
      void performDelete(item);
    },
  });
};

/**
 * 进入游玩。
 */
const play = (item: RecordsListItem) => {
  sessionStorage.setItem('mg_play_entry', 'owner');
  sessionStorage.setItem('mg_owner_play_id', item.requestId);
  router.push(`/play/${item.requestId}`);
};

const design = (item: RecordsListItem) => {
  if (securityLocked.value) {
    showToast(
      '检测到本地模型配置已被修改，已禁用设计功能（数据安全）',
      'error',
    );
    return;
  }
  sessionStorage.setItem('mg_play_entry', 'owner');
  router.push(`/design?id=${item.requestId}`);
};

/**
 * 复制分享链接。
 */
const copyLink = async (item: RecordsListItem) => {
  try {
    const link = `${window.location.origin}/play/${item.requestId}`;
    await navigator.clipboard.writeText(link);
    showToast('链接已复制', 'success');
  } catch (e) {
    console.error(e);
    showToast('复制失败', 'error');
  }
};

/**
 * 执行分享状态切换。
 */
const performToggleShare = async (item: RecordsListItem, next: boolean) => {
  busyItemId.value = item.id;
  try {
    await shareGame(item.requestId, next);
    showToast(next ? '已重新分享' : '已取消分享', 'success');
    await refresh();
  } catch (e: unknown) {
    console.error(e);
    showToast(e instanceof Error ? e.message : '操作失败', 'error');
  } finally {
    busyItemId.value = null;
  }
};

/**
 * 切换分享状态（取消分享 / 重新分享）。
 */
const toggleShare = async (item: RecordsListItem) => {
  if (securityLocked.value) {
    showToast(
      '检测到本地模型配置已被修改，已禁用分享功能（数据安全）',
      'error',
    );
    return;
  }

  if (busyItemId.value) return;

  const next = !item.shared;
  if (!next) {
    openConfirm({
      title: '取消分享',
      message: '确定要取消分享吗？取消后分享链接将不可访问。',
      confirmText: '取消分享',
      cancelText: '返回',
      kind: 'danger',
      onConfirm: () => {
        void performToggleShare(item, next);
      },
    });
    return;
  }

  await performToggleShare(item, next);
};

onMounted(() => {
  refresh();
});

watch(
  uniqueIds,
  () => {
    refresh();
  },
  { deep: true },
);
</script>

<template>
  <div class="relative min-h-screen w-full overflow-hidden bg-black text-white">
    <WavyBackground
      container-class="fixed inset-0 z-0 pointer-events-none"
      :colors="['#38bdf8', '#818cf8', '#c084fc', '#e879f9', '#22d3ee']"
      :waveWidth="110"
      :blur="24"
      speed="fast"
    />

    <div class="relative z-10 mx-auto w-full max-w-5xl px-4 md:px-6 py-8 md:py-10">
      <header class="flex items-start justify-between gap-4">
        <div class="flex items-start gap-3">
          <button
            @click="goHome"
            class="group relative inline-flex items-center justify-center w-11 h-11 rounded-2xl border border-white/10 bg-black/35 backdrop-blur-md hover:bg-black/55 transition-all shadow-[0_0_18px_rgba(168,85,247,0.18)] overflow-hidden"
            title="返回首页"
          >
            <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
            <ArrowLeft class="w-5 h-5 text-white/80 relative z-10" />
          </button>

          <div>
            <div class="text-xs tracking-[0.28em] uppercase text-white/50 font-semibold">Records</div>
            <h1 class="mt-2 text-3xl md:text-5xl font-black bg-gradient-to-r from-purple-200 via-fuchsia-300 to-cyan-200 bg-clip-text text-transparent">
              历史记录
            </h1>
            <p class="mt-2 text-sm text-white/55 max-w-xl leading-relaxed">
              仅保存你分享过的剧情（本地保存 shared_records 的 ID 列表）。你可以从这里继续游玩、取消分享或仅在本地移除。
            </p>
          </div>
        </div>

        <button
          @click="refresh"
          :disabled="isLoading"
          class="group relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all gap-2 overflow-hidden disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <div class="absolute inset-0 bg-white/10 translate-y-full group-hover:translate-y-0 transition-transform duration-300"></div>
          <RefreshCw :class="['w-4 h-4 relative z-10', isLoading ? 'animate-spin' : '']" />
          <span class="relative z-10">刷新</span>
        </button>
      </header>

      <Transition
        enter-active-class="animate-in fade-in slide-in-from-top-4 duration-300"
        leave-active-class="animate-out fade-out slide-out-to-top-2 duration-200"
      >
        <div
          v-if="toast"
          class="fixed top-6 left-1/2 -translate-x-1/2 z-[100] px-4 py-2.5 rounded-2xl border backdrop-blur-md shadow-2xl"
          :class="[
            toast.kind === 'success'
              ? 'border-green-500/30 bg-green-500/10 text-green-100'
              : toast.kind === 'error'
                ? 'border-red-500/30 bg-red-500/10 text-red-100'
                : 'border-white/15 bg-black/35 text-white/90',
          ]"
        >
          <div class="text-sm font-semibold">{{ toast.text }}</div>
        </div>
      </Transition>

      <div class="mt-8">
        <div v-if="error" class="rounded-2xl border border-red-500/30 bg-red-500/10 p-5 backdrop-blur-md">
          <div class="text-sm font-bold text-red-200">加载失败</div>
          <div class="mt-1 text-sm text-white/70">{{ error }}</div>
        </div>

        <div v-else-if="!isLoading && items.length === 0" class="relative overflow-hidden rounded-3xl border border-white/10 bg-black/35 backdrop-blur-xl p-10 shadow-2xl">
          <div class="absolute -inset-1 bg-gradient-to-r from-purple-600/25 via-fuchsia-600/25 to-cyan-600/25 blur-2xl opacity-60"></div>
          <div class="relative">
            <div class="text-sm tracking-[0.24em] uppercase text-white/55 font-semibold">Empty</div>
            <div class="mt-4 text-2xl md:text-3xl font-black text-white/90">还没有历史记录</div>
            <div class="mt-3 text-white/60 leading-relaxed max-w-2xl">
              生成剧情后点击「分享剧情」，系统会创建一条 shared_records 数据，并把它的 ID 保存到你的浏览器中。
            </div>
            <button
              @click="goHome"
              class="mt-6 group relative inline-flex items-center justify-center px-8 py-3 rounded-2xl font-bold text-black bg-white hover:bg-neutral-200 transition-all shadow-[0_0_20px_rgba(255,255,255,0.28)] overflow-hidden"
            >
              <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/50 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700"></div>
              <span class="relative z-10">返回首页去生成</span>
            </button>
          </div>
        </div>

        <div v-else class="space-y-4">
          <TransitionGroup name="records">
            <div
              v-for="item in items"
              :key="item.id"
              class="group relative overflow-hidden rounded-3xl border border-white/10 bg-black/35 backdrop-blur-xl shadow-2xl"
            >
              <div class="absolute -inset-0.5 opacity-0 group-hover:opacity-100 transition-opacity duration-700 bg-gradient-to-r from-purple-600/40 via-fuchsia-600/40 to-cyan-600/40 blur"></div>

              <div class="relative p-6 md:p-7">
                <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-4">
                  <div class="min-w-0">
                    <div class="flex items-center gap-2 flex-wrap">
                      <div
                        class="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs font-mono text-white/65"
                      >
                        <span
                          :class="[
                            'w-2 h-2 rounded-full',
                            item.shared
                              ? 'bg-green-500 shadow-[0_0_10px_rgba(34,197,94,0.55)]'
                              : 'bg-red-500 shadow-[0_0_10px_rgba(239,68,68,0.55)]',
                          ]"
                        ></span>
                        <span>{{ item.shared ? 'Public' : 'Private' }}</span>
                      </div>

                      <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/65">
                        <span class="font-mono">{{ item.playCount }}</span>
                        <span class="text-white/50">次游玩</span>
                      </div>

                      <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/65">
                        <span class="text-white/50">分享时间</span>
                        <span class="font-mono">{{ formatSharedAt(item.sharedAt) }}</span>
                      </div>
                    </div>

                    <div class="mt-4 text-2xl md:text-3xl font-black text-white/90 truncate">
                      {{ item.title || 'Untitled' }}
                    </div>

                    <div class="mt-3 flex flex-wrap gap-2">
                      <span
                        v-for="tag in deriveTags(item)"
                        :key="tag"
                        class="px-3 py-1 rounded-full border border-white/10 bg-white/5 text-xs text-white/70 hover:bg-white/10 transition"
                      >
                        {{ tag }}
                      </span>
                    </div>

                    <div class="mt-4 text-sm text-white/65 leading-relaxed line-clamp-3">
                      {{ item.synopsis || '（暂无剧情简介）' }}
                    </div>
                  </div>

                  <div class="flex md:flex-col gap-2 md:gap-3 md:items-stretch md:w-[190px]">
                    <button
                      @click="play(item)"
                      class="group/btn relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-black bg-white hover:bg-neutral-200 transition-all shadow-[0_0_18px_rgba(255,255,255,0.25)] overflow-hidden flex-1"
                    >
                      <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/50 to-transparent -translate-x-full group-hover/btn:translate-x-full transition-transform duration-700"></div>
                      <Play class="w-4 h-4 relative z-10" />
                      <span class="ml-2 relative z-10">进入游玩</span>
                    </button>

                    <button
                      @click="copyLink(item)"
                      class="group/btn relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all gap-2 overflow-hidden flex-1"
                    >
                      <div class="absolute inset-0 bg-white/10 translate-y-full group-hover/btn:translate-y-0 transition-transform duration-300"></div>
                      <LinkIcon class="w-4 h-4 relative z-10" />
                      <span class="relative z-10">复制链接</span>
                    </button>

                    <button
                      @click="design(item)"
                      :disabled="securityLocked"
                      class="group/btn relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/25 hover:bg-black/45 backdrop-blur-md transition-all gap-2 overflow-hidden flex-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      <div class="absolute inset-0 bg-white/10 translate-y-full group-hover/btn:translate-y-0 transition-transform duration-300"></div>
                      <Pencil class="w-4 h-4 relative z-10" />
                      <span class="relative z-10">进入设计</span>
                    </button>

                    <button
                      @click="toggleShare(item)"
                      :disabled="busyItemId === item.id || securityLocked"
                      class="group/btn relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-white/10 bg-black/35 hover:bg-black/55 backdrop-blur-md shadow-[0_0_25px_rgba(34,211,238,0.14)] transition-all gap-2 overflow-hidden disabled:opacity-50 disabled:cursor-not-allowed flex-1"
                    >
                      <div class="absolute inset-0 bg-white/10 translate-y-full group-hover/btn:translate-y-0 transition-transform duration-300"></div>
                      <Share2 v-if="!item.shared" class="w-4 h-4 relative z-10" />
                      <Lock v-else class="w-4 h-4 relative z-10" />
                      <span class="relative z-10">{{ item.shared ? '取消分享' : '重新分享' }}</span>
                    </button>

                    <button
                      @click="deleteRemote(item)"
                      :disabled="busyItemId === item.id"
                      class="group/btn relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/90 border border-red-500/25 bg-red-500/10 hover:bg-red-500/15 backdrop-blur-md transition-all gap-2 overflow-hidden flex-1 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      <div class="absolute inset-0 bg-white/10 translate-y-full group-hover/btn:translate-y-0 transition-transform duration-300"></div>
                      <Trash2 class="w-4 h-4 relative z-10" />
                      <span class="relative z-10">删除剧情</span>
                    </button>

                    <button
                      @click="requestRemoveLocal(item)"
                      class="group/btn relative inline-flex items-center justify-center px-4 py-3 rounded-2xl font-bold text-white/70 border border-white/10 bg-black/25 hover:bg-black/35 backdrop-blur-md transition-all gap-2 overflow-hidden flex-1"
                    >
                      <div class="absolute inset-0 bg-white/10 translate-y-full group-hover/btn:translate-y-0 transition-transform duration-300"></div>
                      <X class="w-4 h-4 relative z-10" />
                      <span class="relative z-10">仅移除列表</span>
                    </button>
                  </div>
                </div>

                <div class="mt-6 flex items-center justify-between gap-3 text-xs text-white/45">
                  <div class="flex items-center gap-2">
                    <Copy class="w-3.5 h-3.5" />
                    <span class="font-mono">ID: {{ item.id }}</span>
                  </div>

                  <div class="font-mono truncate max-w-[60%]">requestId: {{ item.requestId }}</div>
                </div>
              </div>
            </div>
          </TransitionGroup>
        </div>
      </div>
      <Transition enter-active-class="animate-in fade-in duration-200" leave-active-class="animate-out fade-out duration-150">
        <div v-if="confirmModal" class="fixed inset-0 z-[60] flex items-center justify-center p-4">
          <div class="absolute inset-0 bg-black/80 backdrop-blur-md" @click="closeConfirm"></div>
          <div class="w-full max-w-md bg-neutral-900/90 border border-white/10 rounded-2xl overflow-hidden shadow-2xl relative z-10">
            <div class="px-5 py-4 flex items-center justify-between border-b border-white/10">
              <div class="font-bold text-white">{{ confirmModal.title }}</div>
              <button @click="closeConfirm" class="p-2 rounded-lg hover:bg-white/5 transition-colors">
                <X class="w-5 h-5 text-white/70" />
              </button>
            </div>

            <div class="p-5 text-sm text-white/70 leading-relaxed whitespace-pre-line">
              {{ confirmModal.message }}
            </div>

            <div class="px-5 py-4 bg-black/20 border-t border-white/10 flex items-center justify-end gap-3">
              <button @click="closeConfirm" class="px-5 py-2 rounded-xl bg-white/10 hover:bg-white/20 text-white/80 font-medium transition-colors">
                {{ confirmModal.cancelText }}
              </button>
              <button
                @click="runConfirm"
                :class="[
                  'px-5 py-2 rounded-xl font-bold transition-colors',
                  confirmModal.kind === 'danger'
                    ? 'bg-red-500/20 hover:bg-red-500/30 text-red-100 border border-red-500/20'
                    : 'bg-white/10 hover:bg-white/20 text-white border border-white/10',
                ]"
              >
                {{ confirmModal.confirmText }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.records-enter-active,
.records-leave-active {
  transition: all 280ms ease;
}

.records-enter-from,
.records-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>
