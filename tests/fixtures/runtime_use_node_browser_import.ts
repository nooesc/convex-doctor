"use node";

import { action } from "convex/server";
import { something } from "convex/browser";

export const task = action({
  args: {},
  handler: async () => something,
});
