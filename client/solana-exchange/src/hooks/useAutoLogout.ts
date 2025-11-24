"use client";

import { apiClient } from "@/lib/api";
import { useCallback, useEffect, useRef } from "react";

/**
 * 사용자 활동이 없을 때 자동 로그아웃하는 훅
 * @param inactiveMinutes 비활성 시간 (분) - 기본값 10분
 */
export function useAutoLogout(inactiveMinutes: number = 10) {
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastActivityRef = useRef<number>(Date.now());

  // 타이머 리셋 함수
  const resetTimer = useCallback(() => {
    lastActivityRef.current = Date.now();

    // 기존 타이머 클리어
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    // 새 타이머 설정 (밀리초 단위)
    const inactiveMs = inactiveMinutes * 60 * 1000;
    timeoutRef.current = setTimeout(() => {
      // 인증 상태 확인 후 로그아웃
      if (apiClient.isAuthenticated()) {
        console.log("자동 로그아웃: 10분간 활동이 없었습니다.");
        apiClient.logout();

        // 페이지 새로고침하여 상태 초기화
        window.location.href = "/";
      }
    }, inactiveMs);
  }, [inactiveMinutes]);

  // 사용자 활동 감지 핸들러
  const handleActivity = useCallback(() => {
    resetTimer();
  }, [resetTimer]);

  useEffect(() => {
    // 인증된 사용자만 자동 로그아웃 체크
    if (!apiClient.isAuthenticated()) {
      return;
    }

    // 초기 타이머 설정
    resetTimer();

    // 사용자 활동 이벤트 리스너 등록
    const events = [
      "mousedown",
      "mousemove",
      "keypress",
      "scroll",
      "touchstart",
      "click",
    ];

    events.forEach((event) => {
      window.addEventListener(event, handleActivity, true);
    });

    // 주기적으로 마지막 활동 시간 체크 (1분마다)
    const checkInterval = setInterval(() => {
      const now = Date.now();
      const inactiveMs = inactiveMinutes * 60 * 1000;
      const timeSinceLastActivity = now - lastActivityRef.current;

      // 비활성 시간 초과 시 로그아웃
      if (timeSinceLastActivity >= inactiveMs && apiClient.isAuthenticated()) {
        console.log("자동 로그아웃: 10분간 활동이 없었습니다.");
        apiClient.logout();
        window.location.href = "/";
      }
    }, 60000); // 1분마다 체크

    // 클린업
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      clearInterval(checkInterval);
      events.forEach((event) => {
        window.removeEventListener(event, handleActivity, true);
      });
    };
  }, [handleActivity, inactiveMinutes, resetTimer]);
}
