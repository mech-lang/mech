{
	tyObject_NodecolonObjectType___eN5aWBTKMAUOkqIV7pjhKw* result;
NIM_BOOL* nimErr_;
{nimErr_ = nimErrorFlag();
	result = NIM_NIL;
	eqdestroy___compact_u68(result);
	if (NIM_UNLIKELY(*nimErr_)) goto BeforeRet_;
	result = (tyObject_NodecolonObjectType___eN5aWBTKMAUOkqIV7pjhKw*) nimNewObj(sizeof(tyObject_NodecolonObjectType___eN5aWBTKMAUOkqIV7pjhKw), NIM_ALIGNOF(tyObject_NodecolonObjectType___eN5aWBTKMAUOkqIV7pjhKw));
	{
		NI TM__RFMps0PksymmAfu49cDWRNg_3;
		tyObject_NodecolonObjectType___eN5aWBTKMAUOkqIV7pjhKw* T5_;
		NI TM__RFMps0PksymmAfu49cDWRNg_4;
		tyObject_NodecolonObjectType___eN5aWBTKMAUOkqIV7pjhKw* T6_;
		if (!(((NI)0) < depth_p0)) goto LA3_;
		if (nimSubInt(depth_p0, ((NI)1), &TM__RFMps0PksymmAfu49cDWRNg_3)) { raiseOverflow(); goto BeforeRet_;
		};
		T5_ = NIM_NIL;
		T5_ = make__compact_u7((NI)(TM__RFMps0PksymmAfu49cDWRNg_3));
		if (NIM_UNLIKELY(*nimErr_)) {eqdestroy___compact_u68(T5_); goto BeforeRet_;}
		eqsink___compact_u79(&(*result).left, T5_);
		if (NIM_UNLIKELY(*nimErr_)) goto BeforeRet_;
		if (nimSubInt(depth_p0, ((NI)1), &TM__RFMps0PksymmAfu49cDWRNg_4)) { raiseOverflow(); goto BeforeRet_;
		};
		T6_ = NIM_NIL;
		T6_ = make__compact_u7((NI)(TM__RFMps0PksymmAfu49cDWRNg_4));
		if (NIM_UNLIKELY(*nimErr_)) {eqdestroy___compact_u68(T6_); goto BeforeRet_;}
		eqsink___compact_u79(&(*result).right, T6_);
		if (NIM_UNLIKELY(*nimErr_)) goto BeforeRet_;
	}
LA3_: ;
	}BeforeRet_: ;
	return result;
}