<template>
    <div class="contributors-page" :style="{ backgroundImage: `url(${bgUrl})` }">
        <h1>{{ title }}</h1>
        <p>{{ description }}</p>
        <div class="contributors-list">
            <a
                v-for="contributor in contributors"
                :key="contributor.id"
                :href="contributor.html_url"
                class="contributor-card"
                target="_blank"
                rel="noopener noreferrer"
            >
                <img :src="contributor.avatar_url" :alt="contributor.login" class="contributor-avatar" />
                <div class="contributor-info">{{ contributor.login }}</div>
            </a>
        </div>
        <div class="contributors-more">
            {{ moreText }}
            <a href="https://github.com/longbridge/gpui-kit/graphs/contributors" target="_blank">
                {{ contributorsLinkText }}
            </a>{{ suffixText }}
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Contributor {
    id: number;
    login: string;
    html_url: string;
    avatar_url: string;
}

const props = defineProps<{
    lang: 'en' | 'zh-CN';
    contributors: Contributor[];
}>();

const isZh = computed(() => props.lang === 'zh-CN');
const title = computed(() => (isZh.value ? "贡献者" : "Contributors"));
const description = computed(() =>
    isZh.value ? "感谢所有为这个项目做出贡献的开发者。" : "Thanks to all the people who have contributed to this project!",
);
const moreText = computed(() =>
    isZh.value ? "这里没有展示全部贡献者，完整列表请查看 GitHub 上的 " : "More contributors not shown here. See the full ",
);
const contributorsLinkText = computed(() => (isZh.value ? "贡献者列表" : "Contributors"));
const bgUrl = `${import.meta.env.BASE_URL}contributors.svg`.replace(/\/+/g, '/');
const suffixText = computed(() => (isZh.value ? "。" : " on GitHub."));
</script>

<style>
.contributors-page {
    padding: 2.5rem 0 4rem;
    background-repeat: no-repeat;
    background-position: top 20px right 20px;
}

.contributors-page h1 {
    margin: 0 0 0.6rem;
    font-size: clamp(1.8rem, 3vw, 2.5rem);
    font-weight: 660;
    letter-spacing: -0.04em;
}

.contributors-page > p {
    margin: 0 0 2rem;
    color: var(--muted-foreground);
    font-size: 1rem;
    line-height: 1.7;
}

.contributors-list {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
}

@media (max-width: 860px) { .contributors-list { grid-template-columns: repeat(3, 1fr); } }
@media (max-width: 540px) { .contributors-list { grid-template-columns: repeat(2, 1fr); } }

.contributor-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 0.85rem 1rem;
    border: 1px solid var(--border);
    border-right: 0;
    border-bottom: 0;
    color: var(--foreground);
    text-align: center;
    text-decoration: none;
    transition: background 140ms ease;
}

.contributor-card:hover { background: var(--secondary); }
.contributor-avatar { width: 3rem; height: 3rem; border-radius: 50%; }
.contributor-info { font-size: 0.82rem; font-weight: 500; }

.contributors-more {
    margin-top: 1.5rem;
    color: var(--muted-foreground);
    font-size: 0.875rem;
}

.contributors-more a {
    color: var(--brand);
    text-decoration: none;
}

.contributors-more a:hover { text-decoration: underline; }
</style>
