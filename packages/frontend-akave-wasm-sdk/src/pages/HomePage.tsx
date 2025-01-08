import { useAccount } from "wagmi";
import { useNavigate } from "react-router";
import { useSessionStorage } from "@uidotdev/usehooks";
import { useEffect } from "react";

const Home = () => {
  const { address, isConnected } = useAccount();
  const navigate = useNavigate();
  const [savedPath, setCurrentPath] = useSessionStorage<string | null>(
    "current-path",
    null,
  );

  useEffect(() => {
    const timeout = setTimeout(() => {
      if (address && isConnected) {
        const pathToGo = savedPath;
        setCurrentPath(null);
        navigate(pathToGo ? pathToGo : "/documents");
      }
    }, 500);
    return () => clearTimeout(timeout);
  }, [address, isConnected, navigate, savedPath, setCurrentPath]);

  return <div>Home</div>;
};

export default Home;
