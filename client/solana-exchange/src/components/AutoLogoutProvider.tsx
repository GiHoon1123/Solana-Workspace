"use client";

import { useAutoLogout } from "@/hooks/useAutoLogout";

/**
 * 자동 로그아웃 기능을 제공하는 Provider 컴포넌트
 * 10분간 활동이 없으면 자동으로 로그아웃 처리
 */
export default function AutoLogoutProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  // 10분 비활성 시 자동 로그아웃
  useAutoLogout(10);

  return <>{children}</>;
}
