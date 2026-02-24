import { query, mutation } from "convex/server";
import { v } from "convex/values";

export const list = query({
  args: { channelId: v.id("channels") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("messages")
      .withIndex("by_channel", (q) => q.eq("channelId", args.channelId))
      .collect();
  },
});

export const send = mutation({
  args: { body: v.string(), channelId: v.id("channels") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error("Not authenticated");
    await ctx.db.insert("messages", {
      body: args.body,
      author: identity.subject,
      channelId: args.channelId,
    });
  },
});
