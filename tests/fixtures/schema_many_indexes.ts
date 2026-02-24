import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  events: defineTable({
    title: v.string(),
    userId: v.string(),
    category: v.string(),
    status: v.string(),
    createdAt: v.number(),
    updatedAt: v.number(),
    priority: v.number(),
    location: v.string(),
    tags: v.array(v.string()),
  })
    .index("by_user", ["userId"])
    .index("by_category", ["category"])
    .index("by_status", ["status"])
    .index("by_created", ["createdAt"])
    .index("by_updated", ["updatedAt"])
    .index("by_priority", ["priority"])
    .index("by_location", ["location"])
    .index("by_user_category", ["userId", "category"])
    .index("by_user_status", ["userId", "status"]),
});
