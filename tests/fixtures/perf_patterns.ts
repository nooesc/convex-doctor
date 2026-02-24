import { query, mutation, action } from "convex/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";

// Action calling runAction from within an action
export const orchestrate = action({
  args: {},
  handler: async (ctx) => {
    const identity = await ctx.auth.getUserIdentity();
    await ctx.runAction(internal.tasks.sendEmail);
    await ctx.runQuery(internal.tasks.getItems);
    await ctx.runMutation(internal.tasks.updateItem);
    await ctx.runQuery(internal.tasks.getOtherItems);
  },
});

// Query calling runQuery from within a query (should use helper)
export const getWithSub = query({
  args: { id: v.id("items") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    const item = await ctx.runQuery(internal.items.getById, { id: args.id });
    return item;
  },
});

// Mutation calling runMutation from within a mutation (should use helper)
export const updateWithSub = mutation({
  args: { id: v.id("items") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    await ctx.runMutation(internal.items.markDone, { id: args.id });
  },
});
