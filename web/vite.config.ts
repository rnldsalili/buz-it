import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  build: {
    outDir: "dist",
    rollupOptions: {
      input: {
        player: resolve(__dirname, "player.html"),
        board: resolve(__dirname, "board.html"),
        host: resolve(__dirname, "host.html"),
      },
    },
  },
});
