<template>
    <div class="apps-page">
        <div class="apps-hero">
            <span class="apps-kicker">{{ copy.kicker }}</span>
            <h1>{{ copy.title }}</h1>
            <p class="apps-lead">{{ copy.lead }}</p>
            <details class="apps-policy">
                <summary>{{ copy.selectionLabel }}</summary>
                <p>{{ copy.selectionPolicy }}</p>
                <p>{{ copy.rankingPolicy }}</p>
            </details>
            <ul class="apps-signals">
                <li><Boxes :size="15" /> {{ copy.signalCount }}</li>
                <li><Monitor :size="15" /> macOS / Windows / Linux</li>
                <li><Github :size="15" /> {{ copy.signalLicense }}</li>
            </ul>
        </div>

        <section v-for="section in sections" :key="section.id" class="apps-section" :aria-labelledby="`apps-${section.id}`">
            <h2 :id="`apps-${section.id}`">{{ section.title }} <span>{{ section.total }}</span></h2>
            <p class="apps-section__lead">{{ section.description }}</p>
            <div v-if="section.id === 'all'" class="apps-browser">
                <div class="apps-toolbar">
                    <label class="apps-search">
                        <span class="apps-field-label">{{ copy.searchLabel }}</span>
                        <span class="apps-search__control">
                            <Search aria-hidden="true" />
                            <input v-model="query" type="search" :placeholder="copy.searchPlaceholder" />
                        </span>
                    </label>
                    <AppSortSelect
                        id="apps-sort"
                        v-model="sort"
                        class="apps-sort"
                        :label="copy.sortLabel"
                        :options="[{ value: 'newest', label: copy.newest }, { value: 'stars', label: copy.mostStars }]"
                    />
                </div>
                <div class="apps-filter" role="group" :aria-label="copy.filterLabel">
                    <button
                        v-for="category in categories"
                        :key="category.id"
                        type="button"
                        class="apps-filter__chip"
                        :aria-pressed="String(category.id === active)"
                        @click="active = category.id"
                    >
                        {{ category.label }}
                        <span class="apps-filter__count">{{ category.count }}</span>
                    </button>
                    <button type="button" class="apps-reset" :disabled="!hasFilters" @click="clearFilters">{{ copy.clearFilters }}</button>
                </div>
                <p class="apps-results sr-only" role="status">{{ copy.results(filteredApps.length) }}</p>
            </div>
            <div class="apps-grid">
                <article v-for="app in section.apps" :key="app.id" class="app-card">
                    <a
                        class="app-card__shot"
                        :href="app.hasReadme ? detailUrl(app.id) : (app.website ?? app.source)"
                        :target="app.hasReadme ? undefined : '_blank'"
                        rel="noopener noreferrer"
                        :aria-label="app.name"
                    >
                        <img :src="app.image" :alt="app.name" loading="lazy" decoding="async" />
                    </a>
                    <div class="app-card__body">
                        <div class="app-card__header">
                            <div class="app-card__identity">
                                <h3 class="app-card__name">{{ app.name }}</h3>
                                <p class="app-card__author">{{ app.author }}</p>
                            </div>
                            <span v-if="app.building" class="app-card__status">{{ copy.building }}</span>
                        </div>
                        <p class="app-card__blurb">{{ app.description }}</p>
                        <div class="app-card__platform-row">
                            <ul class="app-card__meta">
                                <li>{{ app.platforms.join(" / ") }}</li>
                            </ul>
                            <time v-if="app.publishedAt" :datetime="app.publishedAt" :title="`${copy.published} ${formatDate(app.publishedAt)}`">{{ formatDate(app.publishedAt) }}</time>
                        </div>
                        <div class="app-card__footer">
                            <div class="app-card__footer-start">
                                <span v-if="!app.source" class="app-card__commercial">{{ copy.commercial }}</span>
                                <span class="app-card__stats inline-flex items-center" role="img" v-if="app.stars !== null" :aria-label="`${app.stars.toLocaleString()} GitHub Stars`" :title="app.starsUpdatedAt ? `${copy.starsUpdated} ${app.starsUpdatedAt.slice(0, 10)}` : undefined"><Star aria-hidden="true" /><span>{{ formatStars(app.stars) }}</span></span>
                            </div>
                            <div class="app-card__links">
                                <a v-if="app.website" class="app-card__icon-link" :href="app.website" :aria-label="`${app.name} — ${copy.visit}`" :title="copy.visit" target="_blank" rel="noopener noreferrer">
                                    <Globe :size="16" aria-hidden="true" />
                                </a>
                                <a v-if="app.source" class="app-card__icon-link" :href="app.source" :aria-label="`${app.name} — ${copy.sourceLink}`" :title="copy.sourceLink" target="_blank" rel="noopener noreferrer">
                                    <Github :size="16" aria-hidden="true" />
                                </a>
                            </div>
                        </div>
                    </div>
                </article>
            </div>
            <nav v-if="section.totalPages > 1" class="apps-pagination" :aria-label="section.id === 'featured' ? copy.featuredPagination : copy.allPagination">
                <span class="apps-pagination__status" role="status">{{ copy.pageStatus(section.page, section.totalPages) }}</span>
                <button type="button" :disabled="section.page === 1" :aria-label="copy.previousPage" :title="copy.previousPage" @click="setPage(section.id, section.page - 1)"><ChevronLeft aria-hidden="true" /></button>
                <button v-for="page in section.totalPages" :key="page" type="button" :aria-label="copy.pageLabel(page)" :aria-current="section.page === page ? 'page' : undefined" @click="setPage(section.id, page)">{{ page }}</button>
                <button type="button" :disabled="section.page === section.totalPages" :aria-label="copy.nextPage" :title="copy.nextPage" @click="setPage(section.id, section.page + 1)"><ChevronRight aria-hidden="true" /></button>
            </nav>
            <div v-if="section.id === 'all' && !filteredApps.length" class="apps-empty">
                <SearchX aria-hidden="true" />
                <h3>{{ copy.empty }}</h3>
                <p>{{ copy.emptyHint }}</p>
                <button type="button" @click="clearFilters">{{ copy.clearFilters }}</button>
            </div>
        </section>

        <div class="apps-cta">
            <h2>{{ copy.ctaTitle }}</h2>
            <p>{{ copy.ctaLead }}</p>
            <a
                class="apps-cta__action"
                href="https://github.com/longbridge/gpui-kit-showcases#submit-an-app"
                target="_blank"
                rel="noopener noreferrer"
            >
                {{ copy.ctaAction }} <ArrowRight :size="15" />
            </a>
        </div>
    </div>
</template>

<script setup lang="ts">
import AppSortSelect from "./AppSortSelect.vue";
import { selectShowcases, paginateFeatured, paginateAll } from "../lib/showcase-browser.ts";
import { computed, ref, watch } from "vue";
import { ArrowRight, Boxes, ChevronLeft, ChevronRight, Github, Globe, Monitor, Search, SearchX, Star } from "lucide-vue-next";

interface ShowcaseApp {
    id: string; name: string; author: string; hasReadme: boolean; category: string; platforms: string[];
    website: string | null; source: string | null; image: string;
    description: string; building: boolean;
    featured: boolean; publishedAt: string | null; stars: number | null; starsUpdatedAt: string | null;
}

const props = defineProps<{ lang: 'en' | 'zh-CN'; apps: ShowcaseApp[] }>();
const apps = props.apps;
const starFormatter = new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 });
const formatDate = (date: string) => date.slice(0, 10).replaceAll("-", "/");
const formatStars = (stars: number) => starFormatter.format(stars);
const detailUrl = (id: string) => `${props.lang === "zh-CN" ? "/zh-CN" : ""}/apps/${id}`;

const isZh = computed(() => props.lang === 'zh-CN');
const locale = computed(() => (isZh.value ? "zh" : "en"));

const CATEGORY_LABELS: Record<string, { en: string; zh: string }> = {
    all: { en: "All", zh: "全部" },
    dev: { en: "Developer Tools", zh: "开发工具" },
    terminal: { en: "Terminal & Network", zh: "终端与网络" },
    system: { en: "System & Desktop", zh: "系统与桌面" },
    work: { en: "Productivity & Media", zh: "效率与媒体" },
};

const active = ref("all");
const query = ref("");
const sort = ref("newest");
const featuredPage = ref(1);
const allPage = ref(1);
watch([query, active, sort], () => { allPage.value = 1; });
const featured = computed(() => paginateFeatured(apps, featuredPage.value));
const hasFilters = computed(() => query.value.trim() !== "" || active.value !== "all");
const clearFilters = () => { query.value = ""; active.value = "all"; };

const categories = computed(() =>
    Object.entries(CATEGORY_LABELS).map(([id, label]) => ({
        id,
        label: label[locale.value as 'en' | 'zh'],
        count: id === "all" ? apps.length : apps.filter((a) => a.category === id).length,
    })),
);

const filteredApps = computed(() => selectShowcases(
    apps,
    { query: query.value, category: active.value, sort: sort.value },
).all);
const all = computed(() => paginateAll(filteredApps.value, allPage.value));
const sections = computed(() => [
    { id: "featured", title: copy.value.featured, description: copy.value.featuredLead, ...featured.value },
    { id: "all", title: copy.value.community, description: copy.value.communityLead, ...all.value },
].filter(section => section.id === "all" || section.apps.length));

function setPage(section: string, page: number) {
    if (section === "featured") featuredPage.value = page;
    else allPage.value = page;
}

const copy = computed(() =>
    isZh.value
        ? { featuredPagination: "精选应用分页", allPagination: "全部应用分页", previousPage: "上一页", nextPage: "下一页", pageLabel: (page: number) => `第 ${page} 页`, pageStatus: (page: number, total: number) => `第 ${page} / ${total} 页`, selectionLabel: "应用收录与精选规则", emptyHint: "试试其他关键词或分类。", starsUpdated: "Stars 更新于", featured: "Featured · 精选应用", featuredLead: "由维护者挑选，展示完整、优质的应用案例。", community: "全部应用", communityLead: "探索社区应用，找到适合你的工具。GitHub Stars 每周及案例 PR 合并后更新。", searchLabel: "搜索应用", searchPlaceholder: "名称、作者、平台…", sortLabel: "应用排序", newest: "最新发布", mostStars: "GitHub Stars 最多", published: "发布于", results: (count: number) => `找到 ${count} 个应用`, empty: "没有找到匹配的应用。", clearFilters: "清除筛选", kicker: "应用案例", title: "用 GPUI Kit 做出来的真实应用。", lead: "探索基于 GPUI Kit 构建的桌面应用，从交易终端、开发工具到日常效率软件。", selectionPolicy: "向 Showcase 仓库提交 PR，审核合并后，应用都会列在 App Stories 中，但不保证进入 Featured。", rankingPolicy: "Featured 由维护者结合项目历史、实现情况、完整度与品质挑选。我们会根据各应用后续更新和整体情况微调名单，尽量展示更完整、有代表性的应用。全部应用可按发布时间或 GitHub Stars 排序，也可搜索。", signalCount: `${apps.length} 个应用`, signalLicense: "开源与商业产品", filterLabel: "按类别筛选", commercial: "商业产品", building: "开发中", visit: "官网", sourceLink: "源码", ctaTitle: "你也用 GPUI Kit 做了应用？", ctaLead: "请在 Showcase 仓库提交 PR，包含应用清单和清晰、完整、整洁的窗口截图。审核合并后自动列在本页，Featured 由维护者另行挑选。", ctaAction: "提交你的应用" }
        : { featuredPagination: "Featured apps pagination", allPagination: "All apps pagination", previousPage: "Previous page", nextPage: "Next page", pageLabel: (page: number) => `Page ${page}`, pageStatus: (page: number, total: number) => `Page ${page} of ${total}`, selectionLabel: "How apps are selected", emptyHint: "Try another keyword or category.", starsUpdated: "Stars updated", featured: "Featured", featuredLead: "Complete, carefully crafted apps selected by the maintainers.", community: "All apps", communityLead: "Explore apps from the community. GitHub Stars refresh weekly and after Showcase PRs merge.", searchLabel: "Search apps", searchPlaceholder: "Name, author, platform…", sortLabel: "Sort apps", newest: "Newest published", mostStars: "Most GitHub Stars", published: "Published", results: (count: number) => `${count} ${count === 1 ? "app" : "apps"} found`, empty: "No apps match your search.", clearFilters: "Clear filters", kicker: "App Stories", title: "Real apps, shipped with GPUI Kit.", lead: "Explore desktop apps built with GPUI Kit, from trading terminals and developer tools to everyday productivity software.", selectionPolicy: "Every app accepted through a merged PR in the Showcase repository is listed in App Stories. A listing does not guarantee a place in Featured.", rankingPolicy: "Maintainers select Featured apps based on project history, implementation, completeness and quality, and revisit the selection as apps evolve to highlight complete, representative examples. Browse all apps by publication date or GitHub Stars, or search the collection.", signalCount: `${apps.length} apps`, signalLicense: "Open source and commercial", filterLabel: "Filter by category", commercial: "Commercial", building: "In development", visit: "Website", sourceLink: "Source", ctaTitle: "Built something with GPUI Kit?", ctaLead: "Open a PR in the Showcase repository with your app manifest and clear, complete, tidy window screenshots. Every merged app PR is published here automatically; maintainers select Featured apps separately.", ctaAction: "Submit your app" },
);
</script>

<style scoped>
.apps-page {
    --apps-space-1: 0.25rem;
    --apps-space-2: 0.5rem;
    --apps-space-3: 0.75rem;
    --apps-space-4: 1rem;
    --apps-space-6: 1.5rem;
    --apps-space-8: 2rem;
    --apps-meta-size: 0.8125rem;
    color: var(--foreground);
}

.apps-hero { max-width: 46rem; margin-bottom: 2.5rem; }
.apps-kicker { display: block; margin-bottom: var(--apps-space-4); color: var(--muted-foreground); font: 600 0.6875rem/1 var(--font-mono); letter-spacing: 0.14em; text-transform: uppercase; }
.apps-hero h1 { margin: 0; padding: 0; border: 0; font-size: clamp(2rem, 3.6vw, 3rem); font-weight: 660; letter-spacing: -0.045em; line-height: 1.15; text-wrap: balance; }
.apps-lead { margin: var(--apps-space-4) 0 0; color: var(--muted-foreground); font-size: 1rem; line-height: 1.65; }
.apps-policy { margin: var(--apps-space-4) 0 0; color: var(--muted-foreground); font-size: 0.875rem; }
.apps-policy summary { width: fit-content; cursor: pointer; border-radius: var(--radius-control); }
.apps-policy summary:hover { color: var(--foreground); }
.apps-policy p { margin: var(--apps-space-3) 0 0; line-height: 1.6; }
.apps-signals { display: flex; flex-wrap: wrap; gap: var(--apps-space-2) var(--apps-space-6); margin: var(--apps-space-4) 0 0; padding: 0; list-style: none; color: var(--muted-foreground); font-size: var(--apps-meta-size); font-variant-numeric: tabular-nums; }
.apps-signals li { display: inline-flex; align-items: center; gap: var(--apps-space-2); margin: 0; line-height: 1.5; }
.apps-signals svg { flex-shrink: 0; width: 1rem; height: 1rem; }

.apps-section + .apps-section { margin-top: 3rem; border-top: 1px solid var(--border); padding-top: var(--apps-space-8); }
.apps-section h2 { display: flex; align-items: baseline; justify-content: space-between; gap: var(--apps-space-4); margin: 0; padding: 0; border: 0; font-size: 1.5rem; font-weight: 620; line-height: 1.3; }
.apps-section h2 span { flex-shrink: 0; color: var(--muted-foreground); font-size: 0.875rem; font-weight: 400; font-variant-numeric: tabular-nums; }
.apps-section__lead { margin: var(--apps-space-2) 0 var(--apps-space-6); color: var(--muted-foreground); font-size: 0.875rem; line-height: 1.5; }
.apps-browser { margin-bottom: var(--apps-space-6); border: 1px solid var(--border); border-radius: var(--radius-card); }
.apps-toolbar { display: flex; flex-wrap: wrap; align-items: end; gap: var(--apps-space-2); padding: var(--apps-space-2); border-bottom: 1px solid var(--border); }
.apps-search { flex: 0 1 20rem; min-width: 0; }
.apps-sort { flex: 0 1 12rem; min-width: 0; }
.apps-field-label { display: block; margin-bottom: 0.375rem; font-size: 0.75rem; font-weight: 550; line-height: 1; }
.apps-search__control { display: block; position: relative; }
.apps-toolbar input { width: 100%; height: 2rem; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--card); color: var(--foreground); padding: var(--apps-space-1) var(--apps-space-2); font: inherit; font-size: var(--apps-meta-size); line-height: 1.5; }
.apps-search__control input { padding-left: 2rem; }
.apps-search__control > svg { position: absolute; top: 50%; transform: translateY(-50%); width: 0.875rem; height: 0.875rem; color: var(--muted-foreground); pointer-events: none; }
.apps-search__control > svg { left: 0.625rem; }
.apps-toolbar input:hover { border-color: var(--brand-line); }
.apps-filter { display: flex; flex-wrap: wrap; align-items: center; gap: var(--apps-space-1); padding: var(--apps-space-2); }
.apps-filter__chip { display: inline-flex; align-items: center; gap: var(--apps-space-2); min-height: 1.75rem; border: 1px solid var(--border); border-radius: var(--radius-control); padding: 0.125rem var(--apps-space-2); color: var(--muted-foreground); font-size: var(--apps-meta-size); line-height: 1.5; cursor: pointer; }
.apps-filter__chip:hover { background: var(--secondary); color: var(--foreground); }
.apps-filter__chip[aria-pressed="true"] { border-color: var(--brand-line); background: var(--secondary); color: var(--foreground); }
.apps-filter__count { font-size: 0.75rem; font-variant-numeric: tabular-nums; }
.apps-results { margin: 0; color: var(--muted-foreground); font-size: var(--apps-meta-size); line-height: 1.5; }
.apps-reset { margin-left: auto; min-height: 1.75rem; border-radius: var(--radius-control); padding: var(--apps-space-1) var(--apps-space-2); color: var(--muted-foreground); font-size: var(--apps-meta-size); cursor: pointer; }
.apps-reset:disabled { opacity: 0.4; cursor: default; }
.apps-reset:hover:not(:disabled) { color: var(--foreground); background: var(--secondary); }
.apps-empty { display: flex; flex-direction: column; align-items: center; gap: var(--apps-space-3); padding: 3rem var(--apps-space-4); border: 1px dashed var(--border); border-radius: var(--radius-card); text-align: center; }
.apps-empty > svg { width: 1.5rem; height: 1.5rem; color: var(--muted-foreground); }
.apps-empty h3 { margin: 0; font-size: 1rem; font-weight: 550; line-height: 1.5; }
.apps-empty p { margin: 0; color: var(--muted-foreground); font-size: 0.875rem; line-height: 1.5; }
.apps-empty button { margin-top: var(--apps-space-1); min-height: 2.25rem; padding: var(--apps-space-2) var(--apps-space-3); border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--card); font-size: 0.875rem; cursor: pointer; }
.apps-empty button:hover { background: var(--secondary); }

.apps-pagination { display: flex; flex-wrap: wrap; align-items: center; justify-content: end; gap: var(--apps-space-1); margin-top: var(--apps-space-4); }
.apps-pagination__status { margin-right: auto; color: var(--muted-foreground); font-size: var(--apps-meta-size); font-variant-numeric: tabular-nums; }
.apps-pagination button { display: inline-flex; align-items: center; justify-content: center; min-width: 2rem; height: 2rem; border: 1px solid transparent; border-radius: var(--radius-control); font-size: var(--apps-meta-size); cursor: pointer; }
.apps-pagination button:hover:not(:disabled) { background: var(--secondary); }
.apps-pagination button[aria-current="page"] { border-color: var(--border); background: var(--secondary); }
.apps-pagination button:disabled { opacity: 0.4; cursor: default; }
.apps-pagination svg { width: 1rem; height: 1rem; }

.apps-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(20rem, 100%), 1fr)); gap: var(--apps-space-4); }
.app-card { display: flex; flex-direction: column; min-width: 0; overflow: hidden; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--card); }
.app-card:hover { border-color: var(--brand-line); }
.app-card__shot { display: block; flex-shrink: 0; border-bottom: 1px solid var(--border); background: var(--secondary); }
.app-card__shot:focus-visible { outline-offset: -2px; }
.app-card__shot img { display: block; width: 100%; aspect-ratio: 16 / 9; object-fit: cover; object-position: top center; }
.app-card__body { display: flex; flex: 1; flex-direction: column; min-width: 0; padding: var(--apps-space-4) var(--apps-space-4) var(--apps-space-2); overflow-wrap: anywhere; }
.app-card__header { display: flex; align-items: start; justify-content: space-between; gap: var(--apps-space-3); }
.app-card__identity { min-width: 0; }
.app-card__name { margin: 0; border: 0; padding: 0; font-size: 1.0625rem; font-weight: 620; letter-spacing: -0.015em; line-height: 1.3; }
.app-card__author { margin: var(--apps-space-1) 0 0; color: var(--muted-foreground); font-size: var(--apps-meta-size); line-height: 1.5; }
.app-card__status { flex-shrink: 0; margin-top: 0; border: 1px solid var(--border); border-radius: var(--radius-control); padding: 0 var(--apps-space-2); color: var(--muted-foreground); font-size: 0.75rem; line-height: 1.75; }
.app-card__blurb { margin: var(--apps-space-3) 0 auto; padding-bottom: var(--apps-space-4); color: var(--foreground); font-size: 0.875rem; line-height: 1.5; }
.app-card__platform-row { display: flex; align-items: baseline; justify-content: space-between; gap: var(--apps-space-2); margin-bottom: var(--apps-space-3); color: var(--muted-foreground); font-size: var(--apps-meta-size); line-height: 1.5; }
.app-card__platform-row > time { flex-shrink: 0; font-variant-numeric: tabular-nums; }
.app-card__meta { min-width: 0; display: flex; flex-wrap: wrap; gap: var(--apps-space-2); margin: 0; padding: 0; list-style: none; }
.app-card__meta li { margin: 0; padding: 0; line-height: inherit; }
.app-card__footer { display: flex; align-items: center; justify-content: space-between; gap: var(--apps-space-2); border-top: 1px solid var(--border); padding-top: var(--apps-space-2); color: var(--muted-foreground); font-size: var(--apps-meta-size); line-height: 1; min-height: 2.5rem; }
.app-card__footer-start { display: flex; flex-wrap: wrap; align-items: center; gap: var(--apps-space-3); min-width: 0; }
.app-card__commercial, .app-card__stats { display: inline-flex; align-items: center; gap: var(--apps-space-1); min-height: 2rem; }
.app-card__stats { font-variant-numeric: tabular-nums; }
/* Numeral ink sits above its line-box center; align it optically with the star. */
.app-card__stats > span { display: inline-flex; align-items: center; height: 1rem; line-height: 1; transform: translateY(0.03125rem); }
.app-card__links { display: flex; flex-shrink: 0; align-items: center; gap: var(--apps-space-1); }
.app-card__icon-link { display: inline-flex; align-items: center; justify-content: center; width: 2rem; height: 2rem; border-radius: var(--radius-control); color: inherit; text-decoration: none; }
.app-card__icon-link:hover { color: var(--foreground); background: var(--secondary); }
.app-card__stats svg, .app-card__links svg { display: block; flex-shrink: 0; width: 1rem; height: 1rem; }

.apps-cta { display: flex; flex-direction: column; align-items: start; gap: var(--apps-space-3); margin-top: 3rem; padding-top: var(--apps-space-8); border-top: 1px solid var(--border); }
.apps-cta h2 { margin: 0; padding: 0; border: 0; font-size: 1.375rem; font-weight: 620; letter-spacing: -0.02em; line-height: 1.3; }
.apps-cta p { margin: 0; max-width: 46rem; color: var(--muted-foreground); font-size: 0.875rem; line-height: 1.6; }
.apps-cta__action { display: inline-flex; align-items: center; gap: var(--apps-space-2); margin-top: var(--apps-space-1); min-height: 2.5rem; border: 1px solid var(--border); border-radius: var(--radius-control); padding: var(--apps-space-2) var(--apps-space-4); color: var(--foreground); background: var(--card); font-size: 0.875rem; font-weight: 500; text-decoration: none; }
.apps-cta__action:hover { background: var(--secondary); }
.apps-cta__action svg { width: 1rem; height: 1rem; }
.apps-page :is(a, button, input, select, summary):focus-visible { outline: 2px solid var(--brand); outline-offset: 2px; }
.apps-page .app-card__shot:focus-visible { outline-offset: -2px; }
.apps-page button:active, .app-card__icon-link:active { background: var(--secondary); color: var(--foreground); }
:global(html[lang^="zh"]) :is(.apps-hero h1, .app-card__name, .apps-cta h2) { letter-spacing: normal; }
:global(html[lang^="zh"]) .apps-kicker { letter-spacing: 0.06em; }
@media (prefers-reduced-motion: no-preference) {
    .app-card { transition: border-color 150ms ease; }
    .app-card__icon-link, .apps-filter__chip, .apps-reset, .apps-cta__action { transition: background-color 150ms ease, color 150ms ease; }
}
@media (max-width: 640px) {
    .apps-search { flex: 1 1 15rem; }
    .apps-sort { flex: 1 1 11rem; }
    .apps-section + .apps-section { margin-top: var(--apps-space-8); }
}
</style>
