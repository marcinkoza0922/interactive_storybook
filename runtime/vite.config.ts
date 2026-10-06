import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [svelte()],
  // Relative asset paths, so a build works from any subpath and from file:// in Electron.
  base: './',
  // public/ holds the development sample book; built runtimes get each book's own bundle.
  build: { copyPublicDir: false },
})
