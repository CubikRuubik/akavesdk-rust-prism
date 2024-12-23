import { NavLink, useLocation } from "react-router";

const Breadcrumbs = () => {
  const { pathname } = useLocation();

  if (pathname === "/") {
    return null;
  }

  const paths = pathname.split("/");

  return (
    <div className="ml-auto mr-auto w-full max-w-[1140px] p-4">
      {paths.map((path, i) => (
        <span key={path}>
          {i !== 0 && <span> {">"} </span>}
          {i === paths.length - 1 ? (
            <span className="capitalize">{path}</span>
          ) : (
            <NavLink
              className={"capitalize"}
              to={paths.slice(0, i + 1).join("/")}
            >
              {path !== "" ? path : "Home"}
            </NavLink>
          )}
        </span>
      ))}
    </div>
  );
};

export default Breadcrumbs;
