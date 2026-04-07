import path from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { createSvgIconsPlugin } from 'vite-plugin-svg-icons'

export default defineConfig(() => {
  const outDir = process.env.YT_PANEL_DIST_OUT_DIR || 'dist'
  const vendorChunkMatchers: Array<[string, string[]]> = [
    ['markdown', ['/markdown-it/', '/highlight.js/', '/katex/', '/@traptitech/markdown-it-katex/']],
    ['utils', ['/fuse.js/', '/dayjs/']],
    ['ui-libs', ['/@vueuse/']],
    ['vue-vendor', ['/vue-router/', '/pinia/', '/vue-i18n/', '/vue/']],
  ]

  return {
    plugins: [
      vue(),
      createSvgIconsPlugin({
        iconDirs: [path.resolve(process.cwd(), 'src/assets/svg-icons')],
        symbolId: '[name]',
      }),
    ],
    resolve: {
      alias: {
        '@': path.resolve(process.cwd(), 'src'),
      },
    },
    server: {
      host: '0.0.0.0',
      port: 3000,
      proxy: {
        '/api': {
          target: 'http://127.0.0.1:3001',
          changeOrigin: true,
        },
      },
    },
    build: {
      outDir,
      emptyOutDir: true,
      reportCompressedSize: false,
      chunkSizeWarningLimit: 1000,
      sourcemap: false,
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (!id.includes('node_modules'))
              return

            for (const [chunkName, matchers] of vendorChunkMatchers) {
              if (matchers.some(matcher => id.includes(matcher)))
                return chunkName
            }
          },
        },
      },
    },
  }
})
