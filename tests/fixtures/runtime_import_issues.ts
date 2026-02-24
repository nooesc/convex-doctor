import { query } from "convex/server";
import { readFileSync } from "node:fs";
import { action } from "convex/node";

export const list = query({
  args: {},
  handler: async () => {
    return readFileSync("/tmp/example.txt", "utf8");
  },
});

export const task = action({
  args: {},
  handler: async () => null,
});
