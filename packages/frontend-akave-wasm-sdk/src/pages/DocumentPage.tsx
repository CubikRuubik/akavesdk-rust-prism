import { useParams } from "react-router";

const DocumentPage = () => {
  const { docId } = useParams();
  return <div>DocumentPage {docId}</div>;
};

export default DocumentPage;
