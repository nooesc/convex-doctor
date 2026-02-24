import { query } from "convex/server";

export const getAll = query(async (ctx) => {
  return await ctx.db.query("items").collect();
});
