import { Outlet, useLocation, useNavigate } from "react-router";
import { useAccount } from "wagmi";
import Spinner from "../components/Spinner";
import { useSessionStorage } from "@uidotdev/usehooks";
import { useEffect } from "react";

const AuthGuardLayout = () => {
  const { isConnected, isConnecting, isReconnecting, isDisconnected } =
    useAccount();
  const location = useLocation();
  const navigate = useNavigate();
  const [, setCurrentPath] = useSessionStorage<string | null>(
    "current-path",
    null,
  );

  const isLoading = !isConnected && (isConnecting || isReconnecting);
  const loggedOut =
    !isConnected && !isConnecting && !isReconnecting && isDisconnected;

  useEffect(() => {
    const timeout = setTimeout(() => {
      // this timeout prevent issues when first render
      if (loggedOut) {
        setCurrentPath(location.pathname);
        console.log("SAVED PATH", location.pathname);
        navigate("/");
      }
    }, 1000);
    return () => clearTimeout(timeout);
  }, [location.pathname, loggedOut, navigate, setCurrentPath]);

  return isLoading || loggedOut ? (
    <div className="flex h-60 w-full items-center justify-center">
      <Spinner className="h-24 w-24" />
    </div>
  ) : (
    <Outlet />
  );
};

export default AuthGuardLayout;
