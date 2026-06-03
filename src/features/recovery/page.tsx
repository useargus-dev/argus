import { Route, Routes } from "react-router-dom";

import { RecoveryCodeStep, RecoveryFactorStep } from "@/features/recovery/code";
import { RecoveryPasswordStep } from "@/features/recovery/password";

export function RecoveryPage() {
  return (
    <Routes>
      <Route index element={<RecoveryCodeStep />} />
      <Route path="password" element={<RecoveryPasswordStep />} />
      <Route path="factor" element={<RecoveryFactorStep />} />
    </Routes>
  );
}
