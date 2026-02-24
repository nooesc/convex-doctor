import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  messages: defineTable({
    body: v.string(),
    author: v.id("users"),
    channelId: v.id("channels"),
  }).index("by_channel", ["channelId"]),
  users: defineTable({
    name: v.string(),
    email: v.string(),
  }),
  channels: defineTable({
    name: v.string(),
  }),
});
