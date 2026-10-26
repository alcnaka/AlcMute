import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

const base = process.env.DOCS_BASE_PATH || '/';
export default defineConfig({
  base,
  outDir: '../output/site',
  trailingSlash: 'always',
  integrations: [starlight({
    title: 'AlcMute',
    description: 'AlcMuteのインストール、操作、設定、トラブル対処の利用ガイド。',
    defaultLocale: 'root',
    locales: { root: { label: '日本語', lang: 'ja' }, en: { label: 'English', lang: 'en' } },
    logo: { src: './src/assets/app.png', replacesTitle: false, alt: '' },
    favicon: '/app.png',
    customCss: ['./src/styles/custom.css'],
    sidebar: [
      { label: 'はじめに', translations: { en: 'Introduction' }, slug: '' },
      { label: 'インストール', translations: { en: 'Installation' }, slug: 'install' },
      { label: '基本操作', translations: { en: 'Basic operation' }, slug: 'usage' },
      { label: '設定', translations: { en: 'Settings' }, slug: 'settings' },
      { label: 'アップデート', translations: { en: 'Updates' }, slug: 'updates' },
      { label: '困ったとき', translations: { en: 'Troubleshooting' }, slug: 'troubleshooting' },
    ],
    lastUpdated: false,
    credits: false,
  })],
});
