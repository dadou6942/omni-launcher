#!c:\users\theod\pycharmprojects\omnilauncher\.venv\scripts\python.exe
# EASY-INSTALL-ENTRY-SCRIPT: 'icoextract==0.1.6','console_scripts','icoextract'
__requires__ = 'icoextract==0.1.6'
import re
import sys
from pkg_resources import load_entry_point

if __name__ == '__main__':
    sys.argv[0] = re.sub(r'(-script\.pyw?|\.exe)?$', '', sys.argv[0])
    sys.exit(
        load_entry_point('icoextract==0.1.6', 'console_scripts', 'icoextract')()
    )
