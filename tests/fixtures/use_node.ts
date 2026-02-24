"use node";

import { action } from "convex/server";

export const sendEmail = action({
  args: {},
  handler: async (ctx) => {
    await ctx.runMutation(api.emails.markSent);
  },
});
