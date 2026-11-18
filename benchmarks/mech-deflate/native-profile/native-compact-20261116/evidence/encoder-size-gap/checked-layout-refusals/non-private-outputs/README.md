# Measure live outputs outside private phases

Apply the pinned compiler patch, rebuild the direct frame-copy variant, and run the replay in this directory. The replay verifies the object, canonical bytecode, and metadata.

```sh
python3 -B replay.py /tmp/mech-frame-copies-71192 /tmp/mech-non-private-outputs
```
