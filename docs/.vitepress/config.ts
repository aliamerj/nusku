// Keep the site settings with the VitePress theme and build configuration.
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join, relative, sep } from 'node:path'
import { defineConfig } from 'vitepress'

const docsRoot = fileURLToPath(new URL('../', import.meta.url))
type Doc = { text: string; link: string; date?: string }

function titleFor(file: string) {
  return file.split(sep).pop()!.replace(/\.md$/, '').replace(/[-_]/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
}

function dateFor(file: string) {
  const frontmatter = readFileSync(file, 'utf8').match(/^---\s*\n([\s\S]*?)\n---(?:\n|$)/)?.[1]
  return frontmatter?.match(/^date:\s*["']?(\d{4}-\d{2}-\d{2})["']?\s*$/m)?.[1]
}

function oldestFirst(a: Doc, b: Doc) {
  if (!a.date && b.date) return 1
  if (a.date && !b.date) return -1
  return (a.date || '').localeCompare(b.date || '') || a.text.localeCompare(b.text)
}

function collect(folder: string): Doc[] {
  const directory = join(docsRoot, folder)
  if (!existsSync(directory)) return []
  const visit = (current: string): string[] => readdirSync(current, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = join(current, entry.name)
    if (entry.isDirectory()) return visit(entryPath)
    return entry.isFile() && entry.name.endsWith('.md') && entry.name !== 'index.md' ? [entryPath] : []
  })
  return visit(directory).map((file) => {
    const slug = relative(directory, file).replace(/\\/g, '/').replace(/\.md$/, '')
    return { text: titleFor(file), link: `/${folder}/${slug}`, date: dateFor(file) }
  }).sort(oldestFirst)
}

const documentation = collect('documentation')
const devlog = collect('devlog')
const base = process.env.GITHUB_REPOSITORY === 'nusku-tools/nusku-tools.github.io' ? '/' : '/nusku/'

export default defineConfig({
  base,
  title: 'Nusku',
  description: 'A continuous profiling system for Linux, built in Zig and eBPF.',
  cleanUrls: true,
  themeConfig: {
    logo: '/mark.svg',
    nav: [
      { text: 'Documentation', link: '/documentation/', activeMatch: '/documentation/' },
      { text: 'Devlog', link: '/devlog/', activeMatch: '/devlog/' }
    ],
    sidebar: {
      '/documentation/': [{
        text: 'Introduction',
        collapsed: false,
        items: documentation.map(({ text, link }) => ({ text, link }))
      }],
      '/devlog/': [{
        text: 'Devlog',
        collapsed: false,
        items: [
          { text: 'All entries', link: '/devlog/' },
          { text: 'Entries', collapsed: false, items: devlog.map(({ text, link }) => ({ text, link })) }
        ]
      }]
    },
    outline: { level: [2, 3], label: 'On this page' },
    search: { provider: 'local' },
    editLink: {
      pattern: 'https://github.com/aliamerj/nusku/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    },
    docFooter: { prev: 'Previous page', next: 'Next page' },
    socialLinks: [{ icon: 'github', link: 'https://github.com/aliamerj/nusku' }],
    footer: { message: 'Built in the open with Zig and eBPF.', copyright: 'Nusku' }
  }
})
