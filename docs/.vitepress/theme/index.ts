import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import CollectionList from './CollectionList.vue'
import './style.css'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('CollectionList', CollectionList)
  }
} satisfies Theme
