<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  folder: 'documentation' | 'devlog'
  empty: string
}>()

type Entry = { title: string; excerpt: string; href: string; date?: string }
const files = import.meta.glob('/{documentation,devlog}/**/*.md', {
  eager: true,
  query: '?raw',
  import: 'default'
}) as Record<string, string>

function titleFrom(path: string) {
  return path.split('/').pop()!.replace(/\.md$/, '').replace(/[-_]/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
}
function dateFrom(source: string) {
  const frontmatter = source.match(/^---\s*\n([\s\S]*?)\n---(?:\n|$)/)?.[1]
  return frontmatter?.match(/^date:\s*["']?(\d{4}-\d{2}-\d{2})["']?\s*$/m)?.[1]
}
function formatDate(date: string) {
  return new Intl.DateTimeFormat('en', { dateStyle: 'medium', timeZone: 'UTC' }).format(new Date(`${date}T00:00:00Z`))
}
function excerptFrom(source: string) {
  const content = source.replace(/^---[\s\S]*?---\s*/m, '')
  const paragraph = content.split(/\n\s*\n/).find((block) => !block.trim().startsWith('#')) || ''
  return paragraph.replace(/[`*_>#\[\]]/g, '').replace(/\([^)]*\)/g, '').trim().slice(0, 180)
}
const entries = computed<Entry[]>(() => Object.entries(files)
  .filter(([path]) => path.startsWith(`/${props.folder}/`) && !path.endsWith('/index.md'))
  .map(([path, source]) => ({
    title: titleFrom(path),
    excerpt: excerptFrom(source),
    href: path.replace(/\.md$/, ''),
    date: dateFrom(source)
  }))
  .sort((a, b) => {
    if (!a.date && b.date) return 1
    if (a.date && !b.date) return -1
    return (a.date || '').localeCompare(b.date || '') || a.title.localeCompare(b.title)
  }))
</script>

<template>
  <div v-if="entries.length" class="collection-grid">
    <a v-for="entry in entries" :key="entry.href" class="collection-card" :href="entry.href">
      <span class="collection-kicker">{{ folder === 'documentation' ? 'Documentation' : 'Devlog entry' }}</span>
      <strong>{{ entry.title }}</strong>
      <time v-if="entry.date" class="collection-date" :datetime="entry.date">{{ formatDate(entry.date) }}</time>
      <span v-if="entry.excerpt" class="collection-excerpt">{{ entry.excerpt }}</span>
      <span class="collection-link">Read more <span aria-hidden="true">→</span></span>
    </a>
  </div>
  <div v-else class="collection-empty">{{ empty }}</div>
</template>
