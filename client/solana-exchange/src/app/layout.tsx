import type { Metadata } from "next";
import Header from "@/components/Header";
import AutoLogoutProvider from "@/components/AutoLogoutProvider";
import "./globals.css";

export const metadata: Metadata = {
  title: "Solana Exchange",
  description: "Solana blockchain trading platform",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="ko">
      <body className="bg-gray-900 text-white min-h-screen">
        <AutoLogoutProvider>
          <Header />
          {children}
        </AutoLogoutProvider>
      </body>
    </html>
  );
}

