const assert=require('node:assert/strict');

const legacyTraits={dynamic:['typing','dynamic'],interpreted:['execution','interpreted'],
  aot:['execution','aot'],jit:['execution','jit'],array:['style','array'],functional:['style','functional']};

function traitSelector(key){
  assert(legacyTraits[key],`No trait migration for ${key}`);
  return `[data-trait-option="${legacyTraits[key].join(':')}"]`;
}

async function click(page,selector){
  await page.evaluate(selector=>{
    const element=document.querySelector(selector);
    if(!element||element.disabled)throw new Error(`Missing or disabled actual control: ${selector}`);
    element.click();
  },selector);
}

async function setTraitChecked(page,key,checked){
  const selector=traitSelector(key);
  await page.evaluate(({selector,checked})=>{
    const input=document.querySelector(selector);
    if(!input||input.disabled)throw new Error(`Missing or disabled actual trait control: ${selector}`);
    if(input.checked!==checked){input.checked=checked;input.dispatchEvent(new Event('change',{bubbles:true}));}
  },{selector,checked});
}

async function selectTraitOnly(page,key){
  const before=await page.evaluate(()=>window.benchmarkExplorer.getSelection());
  // The previous Only buttons replaced manual language selection. Preserve that
  // contract through All plus one facet, using the actual production handlers.
  await click(page,'#all-languages');
  await setTraitChecked(page,key,true);
  const after=await page.evaluate(()=>window.benchmarkExplorer.getSelection());
  assert.deepEqual(after.benchmarks,before.benchmarks,'Trait-only controls preserve requested benchmarks');
  assert.equal(after.releaseYear,before.releaseYear,'Trait-only controls preserve the release year');
  assert.deepEqual(after.disabled,before.disabled,'Trait-only controls preserve individual program exclusions');
  assert.deepEqual(after.ellipses,before.ellipses,'Trait-only controls preserve contour preferences');
}

function expectedTraitKeys(data,key){
  assert(legacyTraits[key],`No independent expected membership for ${key}`);
  const group=data.language_groups.find(group=>group.key===key);
  assert(group,`Recorded source category missing: ${key}`);
  const members=new Set(group.languages);
  // These are independent recorded runtime-policy corrections, not a call to
  // the production trait matcher under test.
  if(key==='interpreted'){members.delete('ruby');members.delete('php');}
  if(key==='jit'){members.add('ruby');members.add('php');}
  return data.languages.filter(language=>members.has(language.key)).map(language=>language.key);
}

module.exports={traitSelector,setTraitChecked,selectTraitOnly,expectedTraitKeys};
