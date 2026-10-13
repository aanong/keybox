import { Database, Globe, KeyRound, Server } from "lucide-react";
import { Category } from "../api";

export function CategoryIcon({ category, size = 16 }: { category: Category; size?: number }) {
  switch (category) {
    case "database":
      return <Database size={size} />;
    case "website":
      return <Globe size={size} />;
    case "server":
      return <Server size={size} />;
    default:
      return <KeyRound size={size} />;
  }
}
