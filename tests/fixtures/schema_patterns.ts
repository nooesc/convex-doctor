import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  posts: defineTable({
    title: v.string(),
    body: v.string(),
    authorId: v.id("users"),
    tags: v.array(v.id("tags")),
    metadata: v.object({
      nested1: v.object({
        nested2: v.object({
          nested3: v.object({
            deep: v.string(),
          }),
        }),
      }),
    }),
  })
    .index("by_author", ["authorId"])
    .index("by_author_title", ["authorId", "title"]),
});
