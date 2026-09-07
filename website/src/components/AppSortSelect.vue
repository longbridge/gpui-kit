<template>
    <div ref="root" class="app-sort" @focusout="onFocusOut">
        <span :id="`${id}-label`" class="app-sort__label">{{ label }}</span>
        <div class="app-sort__control">
            <button
                ref="trigger"
                class="app-sort__trigger"
                type="button"
                aria-haspopup="listbox"
                :aria-expanded="open"
                :aria-controls="`${id}-options`"
                :aria-labelledby="`${id}-label ${id}-value`"
                @click="open ? close() : show()"
                @keydown="onTriggerKey"
            >
                <span :id="`${id}-value`">{{ selectedLabel }}</span>
                <ChevronDown aria-hidden="true" />
            </button>
            <div
                v-if="open"
                :id="`${id}-options`"
                ref="listbox"
                class="app-sort__options"
                :data-side="opensAbove ? 'top' : 'bottom'"
                role="listbox"
                tabindex="-1"
                :aria-labelledby="`${id}-label`"
                :aria-activedescendant="`${id}-option-${activeIndex}`"
                @keydown="onListKey"
            >
                <div
                    v-for="(option, index) in options"
                    :id="`${id}-option-${index}`"
                    :key="option.value"
                    class="app-sort__option"
                    role="option"
                    :aria-selected="option.value === modelValue"
                    :data-active="index === activeIndex"
                    @pointermove="activeIndex = index"
                    @pointerdown.prevent
                    @click="choose(index)"
                >
                    <Check :class="{ 'app-sort__check--hidden': option.value !== modelValue }" aria-hidden="true" />
                    {{ option.label }}
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { Check, ChevronDown } from "lucide-vue-next";

const props = defineProps<{
    id: string;
    label: string;
    modelValue: string;
    options: { value: string; label: string }[];
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
const root = ref<HTMLElement>();
const trigger = ref<HTMLButtonElement>();
const listbox = ref<HTMLElement>();
const open = ref(false);
const opensAbove = ref(false);
const activeIndex = ref(0);
const selectedLabel = computed(() => props.options.find(option => option.value === props.modelValue)?.label ?? "");

async function show(index = props.options.findIndex(option => option.value === props.modelValue)) {
    if (!props.options.length) return;
    activeIndex.value = Math.max(0, index);
    const bounds = trigger.value?.getBoundingClientRect();
    // Match the relative row geometry when deciding which viewport edge has room.
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
    const menuHeight = (props.options.length * 2 + 0.75) * rem;
    opensAbove.value = !!bounds && innerHeight - bounds.bottom < menuHeight && bounds.top > menuHeight;
    open.value = true;
    await nextTick();
    if (open.value) listbox.value?.focus();
}
function close(restoreFocus = false) {
    open.value = false;
    if (restoreFocus) trigger.value?.focus();
}
function choose(index: number) {
    const option = props.options[index];
    if (!option) return;
    emit("update:modelValue", option.value);
    close(true);
}
function onTriggerKey(event: KeyboardEvent) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    show(event.key === "Home" ? 0 : event.key === "End" ? props.options.length - 1 : undefined);
}
function onListKey(event: KeyboardEvent) {
    if (event.key === "Tab") { close(true); return; }
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); close(true); return; }
    if (["Enter", " "].includes(event.key)) { event.preventDefault(); choose(activeIndex.value); return; }
    const count = props.options.length;
    if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
        event.preventDefault();
        activeIndex.value = event.key === "Home" ? 0 : event.key === "End" ? count - 1
            : (activeIndex.value + (event.key === "ArrowDown" ? 1 : -1) + count) % count;
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        const index = props.options.findIndex(option => option.label.toLocaleLowerCase().startsWith(event.key.toLocaleLowerCase()));
        if (index !== -1) activeIndex.value = index;
    }
}
function onOutsidePointer(event: PointerEvent) {
    if (!root.value?.contains(event.target as Node)) close();
}
function onFocusOut(event: FocusEvent) {
    if (!root.value?.contains(event.relatedTarget as Node | null)) close();
}
function onResize() { if (open.value) close(true); }
onMounted(() => {
    document.addEventListener("pointerdown", onOutsidePointer);
    window.addEventListener("resize", onResize);
});
onUnmounted(() => {
    document.removeEventListener("pointerdown", onOutsidePointer);
    window.removeEventListener("resize", onResize);
});
</script>

<style scoped>
.app-sort { min-width: 0; }
.app-sort__control { position: relative; }
.app-sort__label { display: block; margin-bottom: 0.375rem; font-size: 0.75rem; font-weight: 550; line-height: 1; }
.app-sort__trigger { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; width: 100%; min-height: 2rem; padding: 0.25rem 0.5rem; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--card); color: var(--foreground); text-align: start; font: inherit; font-size: 0.8125rem; line-height: 1.5; cursor: pointer; }
.app-sort__trigger:hover, .app-sort__trigger[aria-expanded="true"] { border-color: var(--brand-line); background: var(--secondary); }
.app-sort__trigger > svg { color: var(--muted-foreground); }
.app-sort__trigger:focus-visible { outline: 2px solid var(--brand); outline-offset: 2px; }
.app-sort__options { position: absolute; z-index: 30; inset-inline: 0; top: calc(100% + 0.25rem); padding: 0.25rem; border: 1px solid var(--border); border-radius: var(--radius-control); background: var(--popover); color: var(--popover-foreground); box-shadow: var(--shadow-panel); outline: none; }
.app-sort__options[data-side="top"] { top: auto; bottom: calc(100% + 0.25rem); }
.app-sort__option { display: flex; align-items: center; gap: 0.5rem; min-height: 2rem; padding: 0.25rem 0.5rem; border-radius: var(--radius-control); font-size: 0.8125rem; line-height: 1.5; cursor: pointer; }
.app-sort__option[data-active="true"] { background: var(--secondary); color: var(--foreground); }
.app-sort__check--hidden { visibility: hidden; }
.app-sort svg { display: block; flex-shrink: 0; width: 0.875rem; height: 0.875rem; }
</style>
