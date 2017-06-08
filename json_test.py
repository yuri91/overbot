#!/usr/bin/env python3

import json
import sys

allin = sys.stdin.read()
input_json = json.loads(allin)

chat_id = input_json["chat"]["id"]

text = "si puo fare!!!"

out = {}
out["chat_id"] = chat_id
out["text"] = text

json.dump(out,sys.stdout)


