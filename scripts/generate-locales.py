"""Development-only offline catalog generation using Argos/CTranslate2 models.

No model or Python dependency is shipped with Computy. Run in the documented
Python container with ctranslate2 and sentencepiece installed; review overrides.
"""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import re
import time
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CACHE = Path(os.environ.get("COMPUTY_TRANSLATION_CACHE", "/tmp/computy-models"))
LANGUAGES = {"en":"en","es":"es","pt-BR":"pt","de":"de","it":"it","nl":"nl","pl":"pl","ru":"ru","uk":"uk","tr":"tr","ar":"ar","hi":"hi","bn":"bn","ur":"ur","id":"id","vi":"vi","th":"th","ko":"ko","ja":"ja","zh-CN":"zh","zh-TW":"zt","fa":"fa","he":"he","sw":"sw"}
MARKERS = re.compile(r"\{\{[^{}]+\}\}|\{[a-zA-Z_][^{}]*\}")

def download(url, destination):
    if destination.exists(): return
    temporary = destination.with_suffix(".download")
    for attempt in range(3):
        try:
            with urllib.request.urlopen(url, timeout=90) as response, temporary.open("wb") as file:
                while chunk := response.read(1024 * 1024): file.write(chunk)
            temporary.replace(destination)
            return
        except Exception:
            temporary.unlink(missing_ok=True)
            if attempt == 2: raise
            time.sleep(2 ** attempt)

def model(package):
    code = package["from_code"]+"_"+package["to_code"]
    archive = CACHE/(code+".argosmodel")
    download(next(link for link in package["links"] if link.startswith("https://")), archive)
    folder = CACHE/code
    if not folder.exists():
        folder.mkdir(parents=True)
        with zipfile.ZipFile(archive) as zipped:
            for member in zipped.infolist():
                if not (folder/member.filename).resolve().is_relative_to(folder.resolve()): raise ValueError("Invalid model archive path")
            zipped.extractall(folder)
    metadata = next(folder.rglob("metadata.json"))
    return code, metadata.parent, hashlib.sha256(archive.read_bytes()).hexdigest()

def translate_texts(folder, texts):
    # Product names are invariant, even in technical paragraphs.
    texts = [text.replace("Computy", "{{computyBrand}}").replace("NDNCI", "{{ndnciBrand}}") for text in texts]
    import ctranslate2
    import sentencepiece
    tokenizer_path = folder/"sentencepiece.model"
    if tokenizer_path.exists():
        processor = sentencepiece.SentencePieceProcessor(model_file=str(tokenizer_path))
        encode = lambda value: processor.encode(value, out_type=str)
        decode = lambda pieces: processor.decode(pieces).replace("▁", " ")
    else:
        from sacremoses import MosesTokenizer, MosesDetokenizer, MosesPunctNormalizer
        from subword_nmt.apply_bpe import BPE
        metadata = json.loads((folder/"metadata.json").read_text())
        tokenizer = MosesTokenizer(metadata["from_code"])
        detokenizer = MosesDetokenizer(metadata["to_code"])
        normalizer = MosesPunctNormalizer(metadata["from_code"])
        bpe_file = (folder/"bpe.model").open(encoding="utf8")
        bpe = BPE(bpe_file)
        encode = lambda value: bpe.segment_tokens(tokenizer.tokenize(normalizer.normalize(value)))
        decode = lambda pieces: detokenizer.detokenize(" ".join(pieces).replace("@@ ", "").split(" "))
    translator = ctranslate2.Translator(str(folder/"model"), device="cpu", compute_type="int8", inter_threads=1, intra_threads=2)
    prefix = json.loads((folder/"metadata.json").read_text()).get("target_prefix", "")
    def batch(values):
        tokens = [encode(value) for value in values]
        results = translator.translate_batch(tokens, target_prefix=[[prefix]]*len(tokens) if prefix else None,
            replace_unknowns=True, max_batch_size=512, batch_type="tokens", beam_size=2,
            max_decoding_length=384, length_penalty=0.2, repetition_penalty=1.15, no_repeat_ngram_size=3)
        return [re.sub(r"\s+", " ", decode(result.hypotheses[0]).removeprefix(prefix)).strip() for result in results]
    translated = batch(texts)
    for i, (original, output) in enumerate(zip(texts, translated)):
        if sorted(MARKERS.findall(original)) != sorted(MARKERS.findall(output)):
            # Preserve interpolation and technical formatting tokens exactly.
            parts = re.split("("+MARKERS.pattern+")", original)
            segments = [part for part in parts if part.strip() and not MARKERS.fullmatch(part)]
            replacements = iter(batch(segments)) if segments else iter([])
            fixed = []
            for part in parts:
                if not part.strip() or MARKERS.fullmatch(part): fixed.append(part)
                else:
                    value = next(replacements)
                    fixed.append((" " if part.startswith(" ") else "")+value+(" " if part.endswith(" ") else ""))
            translated[i] = "".join(fixed)
        if not translated[i]: translated[i] = original
    return [text.replace("{{computyBrand}}", "Computy").replace("{{ndnciBrand}}", "NDNCI") for text in translated]

def main():
    CACHE.mkdir(parents=True,exist_ok=True)
    existing_lock = ROOT/"scripts/locales-models.lock.json"
    pinned = json.loads(existing_lock.read_text()) if existing_lock.exists() else {}
    index = []
    if not pinned:
        index_file = CACHE/"index.json"
        download("https://raw.githubusercontent.com/argosopentech/argospm-index/main/index.json", index_file)
        index = json.loads(index_file.read_text())
    packages = {}
    for language,target in LANGUAGES.items():
        if language in pinned:
            packages[language] = pinned[language]["package"]
            continue
        matches = [p for p in index if p["from_code"] == ("fr" if target == "en" else "en") and p["to_code"] == target]
        if not matches: raise ValueError("No offline model for "+language)
        packages[language] = matches[-1]
    paths = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        futures = {pool.submit(model, package): language for language,package in packages.items()}
        for future in concurrent.futures.as_completed(futures):
            language=futures[future];paths[language]=future.result()
            if language in pinned and paths[language][2] != pinned[language]["sha256"]:
                raise ValueError("Translation model checksum mismatch: " + language)
            print("Model ready:",language,flush=True)
    lock = {language:{"package":packages[language],"sha256":paths[language][2]} for language in LANGUAGES}
    (ROOT/"scripts/locales-models.lock.json").write_text(json.dumps(lock,ensure_ascii=False,indent=2)+"\n")
    source = json.loads((ROOT/"src/locales/source.json").read_text())
    output_file = ROOT/"src/locales/catalogs.json"
    catalogs = json.loads(output_file.read_text()) if output_file.exists() else {}
    overrides_file=ROOT/"src/locales/overrides.json"
    overrides=json.loads(overrides_file.read_text()) if overrides_file.exists() else {}
    def degenerate(key,value):
        words=value.split();grams=[" ".join(words[i:i+3]) for i in range(max(0,len(words)-2))]
        return len(value)>max(150,len(key)*5) or any(grams.count(gram)>3 for gram in set(grams)) or bool(re.search(r"(.{1,6})\1{5,}",value)) or "▁" in value
    invariants = {key for key in source if re.fullmatch(r"[\d\W]+|NDNCI|Computy|computy|CPU|RAM|SSD|HDD|NVMe|SMART|SMBIOS|DDR\d|DIMM|SODIMM|MT/s|Mio|Gio|Kio|Tio|ms|s|EXT4",key)}
    for language in LANGUAGES:
        catalog = catalogs.setdefault(language,{})
        catalog.update({key:key for key in invariants})
        catalog.update(overrides.get(language,{}))
        if language == "zh-TW":
            from opencc import OpenCC
            converter=OpenCC('s2twp')
            catalog.update({key:converter.convert(value) for key,value in catalogs['zh-CN'].items()})
            catalog.update(overrides.get(language,{}))
            output_file.write_text(json.dumps(catalogs,ensure_ascii=False,indent=2)+"\n")
            print("Traditional Chinese: converted from reviewed Simplified Chinese catalog.",flush=True)
            continue
        missing = [key for key in source if key not in catalog or (key not in overrides.get(language,{}) and (degenerate(key,catalog[key]) or ("Computy" in key and "Computy" not in catalog[key])))]
        texts = [key if language == "en" else catalogs["en"][key] for key in missing]
        if missing:
            started=time.monotonic()
            values=translate_texts(paths[language][1],texts)
            catalog.update(zip(missing,values))
            output_file.write_text(json.dumps(catalogs,ensure_ascii=False,indent=2)+"\n")
            print(f"Translated {language}: {len(missing)} messages in {time.monotonic()-started:.1f}s",flush=True)
    if overrides_file.exists():
        for language,overrides in json.loads(overrides_file.read_text()).items(): catalogs.setdefault(language,{}).update(overrides)
    output_file.write_text(json.dumps(catalogs,ensure_ascii=False,indent=2)+"\n")
    print("Catalogs complete.",flush=True)

if __name__ == "__main__": main()
