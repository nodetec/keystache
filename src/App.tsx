import { Toaster } from "~/components/ui/sonner";
import { Route, Routes } from "react-router-dom";
import LoginPage from "~/pages/LoginPage";
import Header from "~/components/header/Header";
import ProtectedRoute from "~/components/auth/ProtectedRoute";
import HomePage from "~/pages/HomePage";

export default function App() {
  return (
    <>
      <Header />
      <div className="mx-8 sm:px-6 lg:px-8">
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route
            path="/"
            element={
              <ProtectedRoute>
                <HomePage />
              </ProtectedRoute>
            }
          />
        </Routes>
        <Toaster />
      </div>
    </>
  );
}

