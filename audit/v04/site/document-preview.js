const tags = new Set(['ARTICLE','SECTION','HEADER','FOOTER','DIV','SPAN','H1','H2','H3','H4','H5','H6','P','STRONG','EM','B','I','S','DEL','MARK','SMALL','SUP','SUB','PRE','CODE','BR','HR','UL','OL','LI','DL','DT','DD','BLOCKQUOTE','TABLE','THEAD','TBODY','TFOOT','TR','TH','TD','CAPTION','FIGURE','FIGCAPTION','OUTPUT','A','IMG','DETAILS','SUMMARY']);
function safeUrl(value) {
  try {return ['http:','https:','mailto:'].includes(new URL(value, document.baseURI).protocol);} catch {return false;}
}

// The canonical renderer owns document structure. The browser keeps its passive
// preview limited to document markup and scopes local navigation to this pane.
export function renderDocumentPreview(html) {
  const template = document.createElement('template'); template.innerHTML = html;
  for (const node of [...template.content.querySelectorAll('*')]) {
    if (!tags.has(node.tagName)) {node.replaceWith(document.createTextNode(node.textContent));continue;}
    for (const attribute of [...node.attributes]) {
      const {name,value} = attribute;
      if (name === 'class') node.className = value.split(/\s+/).filter(name => /^(mech-[a-z0-9_-]+|no-output|hidden)$/i.test(name)).join(' ');
      else if (name === 'id') node.id = `document-preview-${value}`;
      else if (name === 'href' && node.tagName === 'A') {
        if (value.startsWith('#')) node.setAttribute(name, `#document-preview-${value.slice(1)}`);
        else if (safeUrl(value)) {node.target='_blank';node.rel='noopener noreferrer';}
        else node.removeAttribute(name);
      } else if (name === 'src' && node.tagName === 'IMG') {
        if (!safeUrl(value) || new URL(value,document.baseURI).protocol==='mailto:') node.removeAttribute(name);
      } else if (!['alt','title','scope','colspan','rowspan','start'].includes(name) && !/^data-mech-[a-z-]+$/.test(name)) node.removeAttribute(name);
    }
  }
  return template.content;
}
