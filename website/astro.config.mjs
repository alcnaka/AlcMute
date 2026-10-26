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
    locales: { root: { label: '日本語', lang: 'ja' } },
    logo: { src: './src/assets/app.png', replacesTitle: false, alt: '' },
    favicon: '/app.png',
    customCss: ['./src/styles/custom.css'],
    sidebar: [
      { label: 'はじめに', slug: '' },
      { label: 'インストール', slug: 'install' },
      { label: '基本操作', slug: 'usage' },
      { label: '設定', slug: 'settings' },
      { label: 'アップデート', slug: 'updates' },
      { label: '困ったとき', slug: 'troubleshooting' },
      { label: 'PDFマニュアル', link: `${base.replace(/\/$/, '')}/AlcMute-Manual.pdf` },
    ],
    lastUpdated: false,
    credits: false,
  })],
});
