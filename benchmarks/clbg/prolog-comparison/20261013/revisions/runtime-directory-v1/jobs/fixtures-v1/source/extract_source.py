"""Extract original source while permitting exact repeated directory identities.

Timestamp metadata is never applied, as in the original extractor. Regular
files and links retain exclusive identities; path aliases are not merged.
"""
from pathlib import Path
import shutil
import tarfile

DIRECTORY_FIELDS = ('name', 'type', 'linkname', 'size', 'mode', 'uid', 'gid', 'uname', 'gname')


def require(value, message):
    if not value: raise ValueError(message)


def directory_identity(member):
    require(member.isdir() and member.type == tarfile.DIRTYPE and member.size == 0 and member.linkname == '',
            'Repeated entry must be an ordinary empty directory')
    return tuple(getattr(member, field) for field in DIRECTORY_FIELDS)


def safe_tree(root):
    root = root.resolve()
    for path in root.rglob('*'):
        if path.is_symlink():
            require(path.resolve(strict=True).is_relative_to(root), 'External bundle symlink')
        else:
            require(path.is_file() or path.is_dir(), 'Unsupported bundle entry')


def extract_source(archive, destination):
    destination = Path(destination)
    destination.mkdir()  # Existing or partially failed destinations never resume.
    root = destination.resolve()
    repeats, selected, names = [], [], {}
    with tarfile.open(archive, 'r:gz') as source:
        for index, member in enumerate(source.getmembers()):
            target = (destination / member.name).resolve()
            require(target.is_relative_to(root), 'Archive member escapes its fresh destination')
            if str(target) in names:
                first_index, first = names[str(target)]
                require(directory_identity(member) == directory_identity(first),
                        'Duplicate file/link, cross-type entry or directory path/metadata alias')
                repeats.append(dict(name=member.name, first_index=first_index, repeat_index=index,
                    first_mtime=first.mtime, repeat_mtime=member.mtime,
                    unapplied_timestamp_difference=first.mtime != member.mtime))
                continue
            names[str(target)] = (index, member)
            require(member.isdir() or member.isfile() or member.issym(), 'Unselected archive member type')
            if member.issym():
                require(not Path(member.linkname).is_absolute() and
                        (target.parent / member.linkname).resolve().is_relative_to(root),
                        'Archive symlink escapes its fresh destination')
            selected.append((target, member))
        # Validate the complete member inventory before writing any source.
        # Defer symlinks until ordinary directories/files are complete, as v2.
        links = []
        for target, member in selected:
            if member.isdir(): target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                with source.extractfile(member) as incoming, target.open('xb') as outgoing:
                    shutil.copyfileobj(incoming, outgoing)
            else: links.append((target, member.linkname))
        for target, link in links:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(link)
    safe_tree(destination)
    return dict(status='completed-original-source-extraction', repeated_directories=repeats,
        repeated_directory_count=len(repeats), directory_identity_fields=list(DIRECTORY_FIELDS),
        timestamps_applied=False, file_or_link_duplicates_accepted=0,
        fresh_destination=True, exclusive_regular_file_writes=True)
