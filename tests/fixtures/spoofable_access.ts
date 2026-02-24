import { mutation } from "convex/server";
import { v } from "convex/values";

export const deleteUser = mutation({
  args: {
    userId: v.id("users"),
    role: v.string(),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.userId, { role: args.role });
  },
});
