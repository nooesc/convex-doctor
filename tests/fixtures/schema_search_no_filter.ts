import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  articles: defineTable({
    title: v.string(),
    body: v.string(),
    category: v.string(),
  })
    .searchIndex("search_body", { searchField: "body" })
    .searchIndex("search_title", {
      searchField: "title",
      filterFields: ["category"],
    }),
});
