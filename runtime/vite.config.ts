import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [svelte()],
  // Relative asset paths, so a build works from any subpath and from file:// in Electron.
  base: './',
})
