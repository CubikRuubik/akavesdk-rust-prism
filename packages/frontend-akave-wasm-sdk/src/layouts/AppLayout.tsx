import { Outlet } from "react-router";
import Header from "../components/Header";
import Breadcrumbs from "../components/Breadcrumbs";

function App() {
  return (
    <>
      <Header />
      <Breadcrumbs />
      <div className="ml-auto mr-auto w-full max-w-[1140px] p-4">
        <Outlet />
      </div>
    </>
  );
}

export default App;
