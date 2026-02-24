import { query, mutation, action } from "convex/server";
import { v } from "convex/values";
import { api } from "./_generated/api";

export const listAll = query({
  handler: async (ctx) => {
    return await ctx.db.query("items").collect();
  },
});

export const filterItems = query({
  args: {},
  handler: async (ctx) => {
    const items = await ctx.db.query("items").filter((q) => q.eq(q.field("status"), "active")).collect();
    const now = Date.now();
    return items;
  },
});

export const processAll = action({
  args: {},
  handler: async (ctx) => {
    const items = await ctx.runQuery(api.items.listAll);
    for (const item of items) {
      await ctx.runMutation(api.items.update, { id: item._id });
    }
    ctx.scheduler.runAfter(0, api.items.cleanup);
  },
});
