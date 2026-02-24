import { action, query } from "convex/server";
import { v } from "convex/values";

export const getMessages = query({
  args: {},
  returns: v.null(),
  handler: async () => null,
});

export const orchestrate = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runQuery(getMessages);
  },
});
