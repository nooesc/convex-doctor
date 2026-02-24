import { mutation } from "convex/server";

export const doSomething = mutation({
  args: {},
  handler: async (ctx) => {
    const apiKey = "sk-1234567890abcdef";
    return apiKey;
  },
});
