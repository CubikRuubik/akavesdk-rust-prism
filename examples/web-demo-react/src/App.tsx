import BlockchainProvider from "./providers/BlockchainProvider";
import { AkaveProvider } from "./providers/AkaveProvider/AkaveProvider";
import Header from "./components/Header";
import { Routes, Route, BrowserRouter, Navigate } from "react-router-dom";
import Home from "./pages/Home";
import NotFound from "./components/NotFound";
import BucketFiles from "./pages/BucketFiles";
import FileInfo from "./pages/FileInfo";

function App() {
  return (
    <BlockchainProvider>
      <AkaveProvider>
        <BrowserRouter>
          <div className="min-h-screen bg-[rgb(var(--color-bg)/1)] text-[rgb(var(--color-text)/1)] transition-colors">
            {/* Top Bar */}
            <Header />
            {/* Main Content */}
            <Routes>
              <Route path="/" element={<Navigate to="/buckets" replace />} />
              <Route path="/buckets" element={<Home />} />
              <Route path="/buckets/:bucketName" element={<BucketFiles />} />
              <Route
                path="/buckets/:bucketName/:fileName"
                element={<FileInfo />}
              />
              <Route path="*" element={<NotFound />} />
            </Routes>
          </div>
        </BrowserRouter>
      </AkaveProvider>
    </BlockchainProvider>
  );
}

export default App;
