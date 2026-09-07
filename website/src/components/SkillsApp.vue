<template>
    <div class="skills-page">
        <h1>{{ title }}</h1>
        <p class="skills-desc">{{ description }}</p>
        <div class="skills-list">
            <div v-for="skill in skills" :key="skill.id" class="skill-card">
                <a
                    :href="`https://github.com/longbridge/gpui-kit/tree/main/${skill.skillPath}`"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="skill-link"
                >
                    <h3 class="skill-name">{{ skill.name }}</h3>
                    <div class="skill-description">{{ skill.description }}</div>
                </a>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Skill {
    id: string;
    name: string;
    description: string;
    skillPath: string;
}

const props = defineProps<{
    lang: 'en' | 'zh-CN';
    skills: Skill[];
}>();

const isZh = computed(() => props.lang === 'zh-CN');
const title = computed(() => (isZh.value ? "GPUI Kit 技能" : "GPUI Kit Skills"));
const description = computed(() =>
    isZh.value
        ? "这里汇总了适用于 GPUI Kit 的开发技能、约定和最佳实践。"
        : "Skills available for working with GPUI Kit. These skills provide guidance and best practices for building GPUI applications.",
);
</script>

<style>
.skills-page h1 {
    margin: 0 0 0.6rem;
    font-size: clamp(1.8rem, 3vw, 2.5rem);
    font-weight: 660;
    letter-spacing: -0.04em;
}

.skills-desc {
    margin: 0 0 2rem;
    color: var(--muted-foreground);
    font-size: 1rem;
    line-height: 1.7;
}

.skills-list { display: flex; flex-direction: column; gap: 0.75rem; }

.skill-card {
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    padding: 0.75rem 1rem;
    transition: background 140ms ease;
}

.skill-card:hover { background: var(--secondary); }

.skill-link {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    color: var(--foreground);
    text-decoration: none;
}

.skill-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 620;
    letter-spacing: -0.015em;
}

.skill-description { font-size: 0.875rem; color: var(--muted-foreground); line-height: 1.6; }
</style>
