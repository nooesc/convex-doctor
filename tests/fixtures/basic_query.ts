import { query, mutation } from "convex/server";
import { v } from "convex/values";

export const getMessages = query({
  args: { channelId: v.id("channels") },
  returns: v.array(v.object({ body: v.string(), author: v.string() })),
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    return await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channelId", args.channelId))
      .collect();
  },
});

export const sendMessage = mutation({
  handler: async (ctx, args) => {
    await ctx.db.insert("messages", { body: args.body });
  },
});
