import { defineConfig } from 'vite';

export default defineConfig({
  base: '/examples/base/',
  build: {
    target: 'esnext',
    minify: true,
    sourcemap: false,
    rollupOptions: {
      onwarn(warning, warn) {
        // wasm-bindgen emits this binding for js_sys::eval. It is generated
        // code; changing it to indirect eval would change scope semantics.
        if (
          warning.code === 'EVAL' &&
          warning.id?.endsWith('/src/wasm/gpui_base_examples_wasm.js')
        ) {
          return;
        }
        warn(warning);
      },
    },
  },
  server: { port: 3001 },
});
