import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./globals.css";
import AppLayout from "./layouts/AppLayout";
import { BrowserRouter, Route, Routes } from "react-router";
import WalletProvider from "./providers/WalletProvider";
import HomePage from "./pages/HomePage";
import DocumentsPage from "./pages/DocumentsPage";
import DocumentPage from "./pages/DocumentPage";
import AuthGuardLayout from "./layouts/AuthGuardLayout";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <WalletProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<HomePage />} />
            <Route path="documents" element={<AuthGuardLayout />}>
              <Route index element={<DocumentsPage />} />
              <Route path=":docId" element={<DocumentPage />} />
            </Route>
          </Route>
        </Routes>
      </BrowserRouter>
    </WalletProvider>
  </StrictMode>,
);
